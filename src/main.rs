pub mod lzw {
    pub mod archive {
        use bitvec::{BigEndian, BitVec};
        use std::collections::HashMap;
        use std::fs::File;
        use std::io::{BufReader, Read, Write};
        use std::path::PathBuf;
        pub struct Data {
            // Словарь, для архивации
            dictionary: HashMap<Vec<u8>, BitVec<BigEndian, u8>>,
            // Номер последнего ключа в словаре
            last_in_dic: u64,
            // Путь к исходному файлу
            source_file: PathBuf,
            // Путь к конечному файлу
            archived_file: PathBuf,
            // Текущее количество бит в максимальном значении словаря
            bits_count: usize,
        }
        impl Data {
            /// Инициализируем структуру начальными значениями
            pub fn new(source_file: &str, archived_file: &str) -> Data {
                let mut dictionary: HashMap<Vec<u8>, BitVec<BigEndian, u8>> = HashMap::new();
                // Инициализируем словарь из всех значений, которые можно хранить
                // в одном байте (0..255)
                for ch in u8::min_value()..=u8::max_value() {
                    dictionary.insert(vec![ch], from_byte(ch));
                }
                Data {
                    // dictionary: test(),
                    // last_in_dic: 4,
                    // bits_count: 3,
                    dictionary,
                    last_in_dic: 255,
                    bits_count: 8,
                    source_file: PathBuf::from(source_file),
                    archived_file: PathBuf::from(archived_file),
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
                let mut prev: Vec<u8> = vec![];
                // Буфер из бит, для добавления в результирующий поток. Сразу выделим память под 30 бит
                let mut bit_buf: BitVec<BigEndian, u8> = BitVec::with_capacity(30);
                // Основной цикл алгоритма. Считываем по одному байту, пока не закончится файл
                while reader.read(&mut buf)? == buf.len() {
                    // Текущий символ
                    let current: u8 = buf[0];
                    // union = prev + current
                    let mut union = prev.clone();
                    union.push(current);
                    // Набор байт уже присутствует в словаре?
                    if self.dictionary.contains_key(&union) {
                        prev = union;
                    } else {
                        // Добавляем P в буфер
                        self.append_to_buf(&mut bit_buf, prev);
                        // Меняем номер последнего ключа в словаре
                        self.last_in_dic += 1;
                        // P + C в словарь
                        self.dictionary
                            .insert(union, from_u64(self.last_in_dic, &mut self.bits_count));
                        // P = C
                        prev = vec![current];
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
                let bv = self.dictionary.get(&value).expect(
                    "Ошибка при получении значения из словаря",
                );
                bit_buf.append(&mut add_leading_zeros(bv, self.bits_count));
            }
        }
        use bitvec::*;
        fn _test() -> HashMap<Vec<u8>, BitVec<BigEndian, u8>> {
            let mut dictionary: HashMap<Vec<u8>, BitVec<BigEndian, u8>> = HashMap::new();
            dictionary.insert(vec![97u8], bitvec![BigEndian, u8; 0, 0, 0]);
            dictionary.insert(vec![98u8], bitvec![BigEndian, u8; 0, 0, 1]);
            dictionary.insert(vec![99u8], bitvec![BigEndian, u8; 0, 1, 0]);
            dictionary.insert(vec![100u8], bitvec![BigEndian, u8; 0, 1, 1]);
            dictionary.insert(vec![101u8], bitvec![BigEndian, u8; 1, 0, 0]);
            dictionary
        }
        /// Добавляет к числу лидирующие нули, добивая его до нужного количества бит
        fn add_leading_zeros(bv: &BitVec<BigEndian, u8>, bits: usize) -> BitVec<BigEndian, u8> {
            let mut moved_bv = bv.clone();
            let bits_to_add = bits - moved_bv.len();
            let mut new_bv = bitvec![BigEndian, u8; 0; bits_to_add];
            new_bv.append(&mut moved_bv);
            new_bv
        }
        /// From byte (u8) to BitVec
        fn from_byte(byte: u8) -> BitVec<BigEndian, u8> {
            // Выделяем память в BitVec под 8 бит
            let mut bv: BitVec<BigEndian, u8> = BitVec::with_capacity(8);
            for i in (0..8).rev() {
                // Добавляем i-ый бит в bv (big-endian ordered)
                bv.push(((1 << i) & byte) != 0);
            }
            bv
        }
        /// Переводит число в BitVec. Обрезает лидирующие нули, однако не больше чем bits
        /// Так же изменяет переданное число bits, сигнализируя о увеличившемся количестве бит в числе
        fn from_u64(value: u64, bits: &mut usize) -> BitVec<BigEndian, u8> {
            let u64_bits_count = 64;
            // Количество бит в числе, без лидирующих нулей
            let mut bits_count: usize = u64_bits_count - value.leading_zeros() as usize;
            //
            if bits_count < *bits {
                bits_count = *bits;
            } else {
                *bits = bits_count;
            }
            let mut bv: BitVec<BigEndian, u8> = BitVec::with_capacity(bits_count);
            for i in (0..bits_count).rev() {
                // Добавляем i-ый бит в bv
                bv.push(((1 << i) & value) != 0);
            }
            bv
        }
        // Получаем из BitVec байты (u8) для записи в файл
        fn pop_byte(bv: &mut BitVec<BigEndian, u8>) -> Option<u8> {
            let byte: u8;
            if bv.len() >= 8 {
                let bv2 = bv.split_off(8);
                byte = bv.as_slice()[0];
                *bv = bv2;
                return Some(byte);
            }
            None
        }
    }
}
fn main() -> std::io::Result<()> {
    lzw::archive::Data::new("test", "output").archive()?;
    Ok(())
}
