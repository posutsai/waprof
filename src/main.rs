extern crate parity_wasm;

// 1. Collect metadata of target function ex. dependencies, types

fn decode_func_id(name_map: &parity_wasm::elements::NameMap, target: &String) -> Option<u32> {
    for (id, func) in name_map.iter() {
        if func == target {
            return Some(id);
        }
    }
    return None;
}

fn identify_dependency(instruc: &parity_wasm::elements::Instructions, name_map: &parity_wasm::elements::NameMap) {
    println!("total {} instructions\n", instruc.elements().len());
    for i in instruc.elements().iter() {
        println!("{}", i);
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

fn inject_call(func_name: String, mut deserialize_mod: parity_wasm::elements::Module) -> parity_wasm::elements::Module {
    let names_section = deserialize_mod.names_section();
    assert_ne!(names_section, None, "The deserialized wasm file should contain names_section use -g2 flag.");
    let import_num = deserialize_mod.import_section().unwrap().functions();
    let name_map = names_section.unwrap().functions().unwrap().names();
    let func_id = decode_func_id(name_map, &func_name);
    let code_section = deserialize_mod.code_section_mut().unwrap();
    let mut instructions = code_section.bodies_mut()[func_id.unwrap() as usize - import_num].code_mut().elements_mut();
    instructions.insert(16, parity_wasm::elements::Instruction::Call(13));
    return deserialize_mod;
}

fn main() {
    let indirect = parity_wasm::deserialize_file("sample/indirect.wasm").expect("Failed to load module").parse_names().unwrap();
    let names_section = indirect.names_section().unwrap();
    search_metadata("addTwo".to_string(), &indirect);

//     let test = parity_wasm::deserialize_file("test.wasm").expect("Failed to load module").parse_names().unwrap();
//     let names_section = test.names_section().unwrap();
//     search_metadata("addTwo".to_string(), &test);
// return;
    let call_manually = parity_wasm::deserialize_file("sample/indirect.wasm").expect("Failed to load module").parse_names().unwrap();
    // let names_section = call_manually.names_section().unwrap();
    let modified_mod = inject_call("addTwo".to_string(), call_manually.clone());
    // let mut build = parity_wasm::builder::from_module(modified_mod);
	// let import_sig = build.push_signature(
	// 	parity_wasm::builder::signature()
	// 		.param().i32()
	// 		.param().i32()
	// 		.return_type().i32()
	// 		.build_sig()
	// );
	// let build = build.import()
	// 	.module("env")
	// 	.field("log")
	// 	.external().func(import_sig)
	// 	.build();

	parity_wasm::serialize_to_file("test.wasm", modified_mod).unwrap();
}
