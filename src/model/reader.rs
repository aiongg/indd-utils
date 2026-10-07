//! The `Reader`: typed access to the objects of a database, with a cache,
//! chunk lookup, child lists and the warnings of a conversion.
//!
//! Evidence: `docs/format/database.md` and `objects.md` (child lists).

use super::*;

/// Reads typed objects from a database, caching them.
pub struct Reader<'a> {
    pub(super) db: &'a Database<'a>,
    pub(super) cache: std::cell::RefCell<HashMap<u32, std::rc::Rc<Object>>>,
    pub(super) warnings: std::cell::RefCell<Vec<String>>,
    /// XML nodes found in the stories read so far.
    pub(super) xml_nodes: std::cell::RefCell<BTreeMap<xml::Key, xml::Node>>,
    /// Text orientation of each text frame read so far, by story.
    pub(super) frame_orientations: std::cell::RefCell<BTreeMap<u32, Vec<Option<Orientation>>>>,
    /// The groups whose page items are being read, outermost first.
    pub(super) item_path: std::cell::RefCell<Vec<u32>>,
}

pub(super) fn uid_or_none(v: u32) -> Option<u32> {
    (v != 0).then_some(v)
}

impl<'a> Reader<'a> {
    pub fn new(db: &'a Database<'a>) -> Reader<'a> {
        Reader {
            db,
            cache: Default::default(),
            warnings: Default::default(),
            xml_nodes: Default::default(),
            frame_orientations: Default::default(),
            item_path: Default::default(),
        }
    }

    pub fn object(&self, uid: u32) -> Result<std::rc::Rc<Object>, Error> {
        if let Some(o) = self.cache.borrow().get(&uid) {
            return Ok(o.clone());
        }
        let obj = self
            .db
            .get(uid)?
            .ok_or_else(|| Error::Corrupt(format!("object {uid} has no data")))?;
        let obj = std::rc::Rc::new(obj);
        crate::audit::object_read(uid);
        self.cache.borrow_mut().insert(uid, obj.clone());
        Ok(obj)
    }

    /// Record a problem that does not stop the conversion.
    pub fn warn(&self, msg: String) {
        self.warnings.borrow_mut().push(msg);
    }

    pub fn class(&self, uid: u32) -> Option<u32> {
        self.db.class_of(uid)
    }

    pub(super) fn chunk(&self, uid: u32, id: u32) -> Result<Option<Vec<u8>>, Error> {
        Ok(self.object(uid)?.chunk(id).map(<[u8]>::to_vec))
    }

    pub(super) fn required(&self, uid: u32, id: u32) -> Result<Vec<u8>, Error> {
        self.chunk(uid, id)?
            .ok_or_else(|| Error::Corrupt(format!("object {uid} has no chunk {id:#x}")))
    }

    pub(super) fn uid_list(&self, uid: u32, id: u32) -> Result<Vec<u32>, Error> {
        match self.chunk(uid, id)? {
            Some(data) => Cursor::new(&data).u32_list(),
            None => Ok(Vec::new()),
        }
    }

    /// Children listed in a hierarchy chunk: parent, owner, then a u32 list.
    pub(super) fn children(&self, uid: u32, id: u32) -> Result<Vec<u32>, Error> {
        match self.chunk(uid, id)? {
            Some(d) => {
                let mut c = Cursor::new(&d);
                c.skip(8)?;
                c.u32_list()
            }
            None => Ok(Vec::new()),
        }
    }
}
