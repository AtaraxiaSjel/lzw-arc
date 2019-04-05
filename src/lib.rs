pub mod lzw {
    pub mod archive {
        use bitvec::{BigEndian, BitVec};
        use std::collections::HashMap;
        use std::fs::File;
        use std::io::{BufReader, Read, Write};
        use std::path::PathBuf;
        pub struct Data {
            // Словарь, для архивации
            dictionary: HashMap<Vec<u8>, u32>,
            // Номер последнего ключа в словаре
            last_in_dic: u32,
            // Путь к исходному файлу
            source_file: PathBuf,
            // Путь к конечному файлу
            archived_file: PathBuf,
            // Текущее количество бит в максимальном значении словаря
            bits_count: u8,
            // Максимальное количество бит, т.е. размер словаря
            max_bits_count: u8,
        }
        impl Data {
            /// Инициализируем структуру начальными значениями
            pub fn new(source_file: &str, archived_file: &str, max_bits_count: u8) -> Data {
                if max_bits_count > 32 || max_bits_count < 9 {
                    panic!("Недопустимый размер словаря! Разрешенный: 9 <= n <= 32");
                }
                if !std::path::Path::new(source_file).exists() {
                    panic!("Исходный файл не существует!");
                }
                let dictionary = reset_dictionary();
                Data {
                    dictionary,
                    last_in_dic: 255,
                    bits_count: 8,
                    source_file: PathBuf::from(source_file),
                    archived_file: PathBuf::from(archived_file),
                    max_bits_count,
                }
            }
            #[cfg(debug_assertions)]
            pub fn new_test(source_file: &str, archived_file: &str, max_bits_count: u8) -> Data {
                if max_bits_count > 32 || max_bits_count < 9 {
                    panic!("Недопустимый размер словаря! Разрешенный: 9 <= n <= 32");
                }
                if !std::path::Path::new(source_file).exists() {
                    panic!("Исходный файл не существует!");
                }
                Data {
                    dictionary: test(),
                    last_in_dic: 4,
                    bits_count: 3,
                    source_file: PathBuf::from(source_file),
                    archived_file: PathBuf::from(archived_file),
                    max_bits_count,
                }
            }
            pub fn archive(&mut self) -> std::io::Result<()> {
                // Открываем исходный файл и подключаем его к буферу
                let source_file = File::open(self.source_file.as_path())?;
                let mut reader = BufReader::new(source_file);
                // Выходной поток
                let mut archived_file = File::create(self.archived_file.as_path())?;
                // Буфер для считываемого байта
                let mut buf = [0u8; 1];
                // Предыдущая строка
                let mut prev: Vec<u8> = Vec::with_capacity(64);
                // Буфер из бит, для добавления в результирующий поток
                let mut bit_buf: BitVec<BigEndian, u8> = BitVec::with_capacity(8);
                // Инициализация. Считываем первый байт
                if reader.read(&mut buf)? != buf.len() {
                    panic!("Передан пустой файл");
                }
                prev.push(buf[0]);
                // Основной цикл алгоритма. Считываем по одному байту, пока не закончится файл
                while reader.read(&mut buf)? == buf.len() {
                    // Текущий символ
                    let current: u8 = buf[0];
                    prev.push(current);
                    // Набор байт уже присутствует в словаре?
                    if !self.dictionary.contains_key(&prev) {
                        // Добавляем P в буфер
                        self.append_to_buf(&mut bit_buf, prev[0..prev.len() - 1].to_vec());
                        // Меняем номер последнего ключа в словаре
                        self.add_element_count();
                        // P + C в словарь
                        self.dictionary.insert(prev.clone(), self.last_in_dic);
                        // P = C
                        prev.clear();
                        prev.push(current);
                        //Проверяем, может ли добавить что-то в файл
                        while let Some(byte) = pop_byte(&mut bit_buf) {
                            archived_file.write_all(&[byte])?;
                        }
                    }
                }
                // Добавляем в буфер оставшиеся байты
                self.append_to_buf(&mut bit_buf, prev);
                let last_bytes: Vec<u8> = bit_buf.as_slice().iter().rev().cloned().collect();
                // Добавляем в файл последние байты, дополняя их нулями
                archived_file.write_all(&last_bytes)?;
                Ok(())
            }
            /// Добавляем в буфер кодовое значение из словаря, для дальнейшего добавления в файл
            fn append_to_buf(&self, bit_buf: &mut BitVec<BigEndian, u8>, value: Vec<u8>) {
                let bv = *self.dictionary.get(&value).expect(
                    "Ошибка при получении значения из словаря",
                );
                bit_buf.append(&mut from_u32(bv, self.bits_count));
            }
            // Увеличиваем счетчик словаря
            fn add_element_count(&mut self) {
                self.last_in_dic += 1;
                let bits_count = 32 - self.last_in_dic.leading_zeros() as u8;
                // Сбрасываем словарь, если достигли максимального количества бит
                if bits_count > self.max_bits_count {
                    self.dictionary = reset_dictionary();
                    self.bits_count = 8;
                    self.last_in_dic = 255;
                } else {
                    self.bits_count = bits_count;
                }
            }
        }
        fn reset_dictionary() -> HashMap<Vec<u8>, u32> {
            let mut dictionary: HashMap<Vec<u8>, u32> = HashMap::new();
            // Инициализируем словарь из всех значений, которые можно хранить
            // в одном байте (0..255)
            for ch in u8::min_value()..=u8::max_value() {
                dictionary.insert(vec![ch], u32::from(ch));
            }
            dictionary
        }
        #[cfg(debug_assertions)]
        fn test() -> HashMap<Vec<u8>, u32> {
            let mut dictionary: HashMap<Vec<u8>, u32> = HashMap::new();
            dictionary.insert(vec![97u8], 0);
            dictionary.insert(vec![98u8], 1);
            dictionary.insert(vec![99u8], 2);
            dictionary.insert(vec![100u8], 3);
            dictionary.insert(vec![101u8], 4);
            dictionary
        }
        /// Переводит число в BitVec. Обрезает лидирующие нули, однако не больше чем bits
        fn from_u32(value: u32, bits: u8) -> BitVec<BigEndian, u8> {
            let mut bv: BitVec<BigEndian, u8> = BitVec::with_capacity(bits as usize);
            for i in (0..bits).rev() {
                // Добавляем i-ый бит в bv
                bv.push(((1 << i) & value) != 0);
            }
            bv
        }
        // Получаем из BitVec байты (u8) для записи в файл
        fn pop_byte(bv: &mut BitVec<BigEndian, u8>) -> Option<u8> {
            let byte: u8;
            if bv.len() > 8 {
                let bv2 = bv.split_off(8);
                byte = bv.as_slice()[0];
                *bv = bv2;
                return Some(byte);
            }
            None
        }
    }
}