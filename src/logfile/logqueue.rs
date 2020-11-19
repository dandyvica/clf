//! Manage log rotation
use crate::logfile::logfile::LogFile;

// A dedicated container aimed at managing rotation of a logfile
pub struct LogQueue<'a> {
    rotated: Option<&'a mut LogFile>,
    plain: &'a mut LogFile,
    index: usize,
}

impl<'a> LogQueue<'a> {
    pub fn new(logfile: &'a mut LogFile) -> Self {
        Self {
            rotated: None,
            plain: logfile,
            index: 0,
        }
    }

    // assign & calculate rotated logfile
    pub fn set_rotated(&mut self, rotated: &'a mut LogFile) {
        self.rotated = Some(rotated);
    }

    /// Returns the number of elements which is either 1 or 2
    pub fn count(&self) -> usize {
        match self.rotated {
            Some(_) => 2,
            None => 1,
        }
    }

    /// Mimics an iteration. Because it was too much a hassle to deal with mutable references, using
    /// standard iterators, and because this struct is only used once, this simple method implements
    /// a `next()` kind of iteration. To be used with `while let` construct.
    pub fn next(&mut self) -> Option<&mut LogFile> {
        match self.index {
            0 => {
                if self.rotated.is_some() {
                    self.index = 1;
                    return Some(self.rotated.as_mut().unwrap());
                } else {
                    self.index = 2;
                    return Some(self.plain);
                }
            }
            1 => {
                self.index = 2;
                return Some(&mut self.plain);
            }
            _ => return None,
        };
    }
}

mod tests {
    use super::*;

    #[test]
    fn iter() {
        let mut a = LogFile::new("/var/log/kern.log").unwrap();
        let mut b = LogFile::new("/var/log/boot.log").unwrap();

        let mut queue = LogQueue::new(&mut a);
        while let Some(x) = queue.next() {
            println!("object1={:?}", x);
        }

        assert_eq!(queue.count(), 1);

        queue = LogQueue::new(&mut a);
        queue.set_rotated(&mut b);
        assert_eq!(queue.count(), 2);

        while let Some(x) = queue.next() {
            x.path = std::path::PathBuf::from("/foo");
            println!("object2={:?}", x);
        }
    }
}
