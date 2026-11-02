use crate::{Manager, Result};
use rusqlite::fallible_iterator::FallibleIterator;
use rusqlite::{Batch, Connection, OpenFlags, types::ValueRef};
use std::{path::Path, time::Instant};
impl Manager {
    pub(crate) fn sqlite_execute(&self, path: &Path, sql: &str, readonly: bool) -> Result<String> {
        let flags = if readonly {
            OpenFlags::SQLITE_OPEN_READ_ONLY
        } else {
            OpenFlags::SQLITE_OPEN_READ_WRITE
        };
        let mut conn = Connection::open_with_flags(path, flags)?;
        conn.busy_timeout(self.timeout)?;
        let deadline = Instant::now();
        let timeout = self.timeout;
        conn.progress_handler(1000, Some(move || deadline.elapsed() > timeout))?;
        let mut batch = Batch::new(&conn, sql);
        let mut output = String::new();
        while let Some(mut statement) = batch.next()? {
            let count = statement.column_count();
            if count == 0 {
                statement.execute([])?;
            } else {
                let mut rows = statement.query([])?;
                while let Some(row) = rows.next()? {
                    let mut fields = Vec::new();
                    for col in 0..count {
                        fields.push(match row.get_ref(col)? {
                            ValueRef::Null => String::new(),
                            ValueRef::Integer(n) => n.to_string(),
                            ValueRef::Real(n) => n.to_string(),
                            ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned(),
                            ValueRef::Blob(b) => b.iter().map(|v| format!("{v:02x}")).collect(),
                        });
                    }
                    output.push_str(&fields.join("|"));
                    output.push('\n');
                }
            }
        }
        Ok(output)
    }
}
