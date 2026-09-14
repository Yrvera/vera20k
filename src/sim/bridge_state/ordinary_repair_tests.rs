use super::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Cell {
    Real(CellCoord),
    Dummy,
}

struct Host {
    cells: BTreeMap<CellCoord, i32>,
    dummy: (CellCoord, i32),
    variant: u8,
    trace: Vec<Value>,
}

impl Host {
    fn coord(&self, cell: Cell) -> CellCoord {
        match cell {
            Cell::Real(c) => c,
            Cell::Dummy => self.dummy.0,
        }
    }
}

impl OrdinaryRepairHost for Host {
    type Cell = Cell;
    type Error = ();
    fn lookup(&mut self, coord: CellCoord) -> Cell {
        // Native fixed-stride aliases, rather than independent X/Y clipping.
        let index = i32::from(coord.1) * 512 + i32::from(coord.0);
        if (0..0x40000).contains(&index) {
            let canonical = ((index % 512) as i16, (index / 512) as i16);
            if self.cells.contains_key(&canonical) {
                return Cell::Real(canonical);
            }
        }
        self.dummy.0 = coord;
        Cell::Dummy
    }
    fn overlay(&self, cell: Cell) -> i32 {
        match cell {
            Cell::Real(c) => self.cells[&c],
            Cell::Dummy => self.dummy.1,
        }
    }
    fn write_overlay(&mut self, cell: Cell, overlay: u8) {
        self.trace
            .push(json!({"kind":"overlay","coord":self.coord(cell),"overlay":overlay}));
        match cell {
            Cell::Real(c) => {
                self.cells.insert(c, i32::from(overlay));
            }
            Cell::Dummy => self.dummy.1 = i32::from(overlay),
        }
    }
    fn variant(&mut self) -> u8 {
        self.trace
            .push(json!({"kind":"random","minimum":0,"maximum":3,"result":self.variant}));
        self.variant
    }
    fn redraw(&mut self, _: Cell) {
        self.trace.push(json!({"kind":"screen"}));
    }
    fn radar(&mut self, coord: CellCoord) {
        self.trace.push(json!({"kind":"radar","coord":coord}));
    }
    fn recalc(&mut self, cell: Cell) -> Result<(), ()> {
        self.trace
            .push(json!({"kind":"recalc","coord":self.coord(cell),"level":-1}));
        Ok(())
    }
    fn occupants(&mut self, cell: Cell) -> Result<(), ()> {
        self.trace
            .push(json!({"kind":"occupants","coord":self.coord(cell),"mode":0}));
        Ok(())
    }
    fn connectivity(&mut self) -> Result<(), ()> {
        self.trace.push(json!({"kind":"connectivity"}));
        Ok(())
    }
    fn rebuild_rectangle(&mut self, [x, y, w, h]: Rect) -> Result<(), ()> {
        let cells: Vec<_> = (x..x + w)
            .flat_map(|x| (y..y + h).map(move |y| [x as i16, y as i16]))
            .collect();
        self.trace.push(json!({"kind":"rebuild","cells":cells}));
        Ok(())
    }
}

#[test]
fn ordinary_repair_matches_original_overlay_and_callback_corpus() {
    let corpus: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tools/spatial_oracle/bridge_ordinary_repair.json"
    )))
    .unwrap();
    for case in corpus["cases"].as_array().unwrap() {
        let input = &case["input"];
        let coord = |row: &Value| {
            (
                row[0].as_i64().unwrap() as i16,
                row[1].as_i64().unwrap() as i16,
            )
        };
        let mut host = Host {
            cells: input["cells"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| (coord(row), row[5].as_i64().unwrap() as i32))
                .collect(),
            dummy: ((0, 0), -1),
            variant: input["variant"].as_u64().unwrap() as u8,
            trace: vec![],
        };
        repair(
            &mut host,
            coord(&input["start"]),
            if input["family"] == "low" {
                Family::Low
            } else {
                Family::High
            },
        )
        .unwrap();
        assert_eq!(
            json!(host.trace),
            case["result"]["trace"],
            "{}",
            input["name"]
        );
        let final_cells: Vec<_> = input["cells"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                let (x, y) = coord(row);
                [i32::from(x), i32::from(y), host.cells[&(x, y)]]
            })
            .collect();
        assert_eq!(
            json!(final_cells),
            case["result"]["final"],
            "{}",
            input["name"]
        );
        assert_eq!(
            json!([
                i32::from(host.dummy.0.0),
                i32::from(host.dummy.0.1),
                host.dummy.1
            ]),
            case["result"]["dummy"],
            "{}",
            input["name"]
        );
    }
}
