use std::fs::File;
use std::io::{BufRead, BufReader, Error, ErrorKind, Read, Seek, SeekFrom};
use std::iter::Iterator;
use std::rc::Rc;

pub struct FileIterator {
    reader: BufReader<File>,
    buffer: Rc<String>,
}

impl Iterator for FileIterator {
    type Item = Rc<String>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = Rc::get_mut(&mut self.buffer).unwrap();
        self.reader.read_line(line).unwrap()
    }
}

