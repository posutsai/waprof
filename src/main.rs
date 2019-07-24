extern crate parity_wasm;

// 1. Collect metadata of target function ex. dependencies, types
pub enum dtype {
    void,
    u32_t,
    i32_t,
}

pub struct Function {
    pub input_types: Vec<dtype>,
    pub output_types: Vec<dtype>,
    pub dependencies: Option<Vec<Function>>,
}

fn decode_func_id(name_map: &parity_wasm::elements::NameMap, target: &String) -> Option<u32> {
    for (id, func) in name_map.iter() {
        if func == target {
            return Some(id);
        }
    }
    return None;
}

fn identify_dependency(instruc: &parity_wasm::elements::Instructions, name_map: &parity_wasm::elements::NameMap) {
    for i in instruc.elements().iter() {
        match i {
            parity_wasm::elements::Instruction::Call(callee) => {
                println!("the dependency is {}", name_map.get(*callee).unwrap());
            },
            _ => {},
        }
    }
}

fn search_metadata(func_name: String, deserialize_mod: &parity_wasm::elements::Module) {
    let mut import_num: usize = 0;
    match deserialize_mod.import_section() {
        Some(sec) => {
            import_num = sec.functions();
        },
        _ => {},
    }
    let names_section = deserialize_mod.names_section();
    assert_ne!(names_section, None, "The deserialized wasm file should contain names_section use -g2 flag.");
    let name_map = names_section.unwrap().functions().unwrap().names();
    let func_id = decode_func_id(name_map, &func_name);
    assert_ne!(func_id, None);
    println!("target function id is {}", func_id.unwrap());
    println!("import num is {}", import_num);

    let code_section = deserialize_mod.code_section().unwrap();
    identify_dependency(code_section.bodies()[func_id.unwrap() as usize - import_num].code(), name_map);
}

fn main() {
    let template = parity_wasm::deserialize_file("sample/template.wasm").expect("Failed to load module").parse_names().unwrap();
    let instrument = parity_wasm::deserialize_file("sample/instrument.wasm").expect("Failed to load module").parse_names().unwrap();
    let names_section = instrument.names_section().unwrap();
    search_metadata("_enterFunc".to_string(), &instrument);
}
