---
tags: mbilab
---
# waprof
- [x] Compile c code to WebAssembly
- [x] Understand WebAssembly binary format
- [x] Binary instrumentation (successfully injected but not workable)
- [ ] Figure out function executing mechanism and memory model
- [ ] Log data and print graph

## Motivation
WebAssembly is designed to run in an independent and isolated environment. The contributions of WebAssembly are performance and safety. However, if we target wasm as our backend, there is no existing tool to compare the dynamic runtime to the native language. That's why we try to identify if there is any unconsistent conditions. According to the runtime implementation, runtime is either JIT compiler or vm. It is possible that the bottleneck of wasm is totally different from native process.

## Tools
In order to achieve our goals, there are two important tools need to be introduced.

Nowadays, the target deploying environments of wasm are not only in browser but also beyond browser. Mozilla already standardize so-called **WebAssembly system interface**. With WASI and all the implementations satisfied the spec, wasm files are able to be executed outside browser. In our profiler, we choose lucet as our experiment subject due to its complete tool-chain.

However, `lucet` is not enough. To inspect `wasm` file conveniently, we use series of tools provided by wabt.
### lucet tool-chain
The main tools we use in lucet tool-chain are `wasm32-wasi-gcc`, `lucetc-wasi` and `lucet-wasi`. As `wasm32-wasi-gcc` its name suggests, it plays the role just like `gcc` and `clang`. `wasm32-wasi-gcc` compiles normal `.c` file into `.wasm` file.

`lucetc-wasi` connect the gap between `wasm` and implemented virtual machine and output an exeutable.

Finally, we take advantage of `lucet-wasi` to instantiate actually VM and create an isolated environment to deploy.
### wabt
`wasm-objdump`
### wasmer
### Note
There are a lot of tutorial sugest to use `emcc` in `emsdk` to perform all pipeline. However, 
## Understand WebAssembly Binary Format
### Observe C function in `wasm` file
#### 0. Create a simple C file
We use simple C code as sample to demo how c-style function convert into wasm. Following code segment shows a simple `addTwo` function in C, wat (WebAssemblyText) and wasm bytecode.

```C
int addTwo(int a, int b) {
    return a + b;
}
```
#### 1. Convert C code into wasm
At first we use `emcc` from [emsdk](https://github.com/emscripten-core/emsdk) to compile C code into wasm bytecode. In order to simplify the process as easy as possible, we remove all the runtime imported function with flag `ONLY_MY_CODE=1`.
`$ emcc -O1 -s WASM=1 -s  ONLY_MY_CODE=1 addTwo.c -o addTwo.wasm`
> Tips:
> Due to our simple example, the compiler may automatically inline our subjet `addTwo` function. To prevent inlining, `clang` offers `optnone` attribute to do so.
> [color=#19fc33]
```C
int __attribute__((optnone)) addTwo(int a, int b) {
    return a + b;
}
```
#### 2. Inspect binary code with official spec
There is a useful tool to inspect wasm binary format simply called [wasmer](https://github.com/WebAssembly/wabt). Actually the repository is a collection of tools including `wasm-objdump` and `wasm2wat`.

With the `wasm` file from former step, we convert it into more readible format.
`$ wasm2wat addTwo.wasm`
The output is shown as following. Obviously `wat` represent the whole function with a series of instructions and it contains 1 module. In this sample we only have one module and the module is divided into multiple sections such as *type section*, *function section* and *export section*. The detail in those section will be explain clearly below.
```
----------------------- WebAssemblyText --------------------------
$ wasm2wat addTwo.wasm
(module
  (type (;0;) (func (param i32 i32) (result i32)))
  (func (;0;) (type 0) (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add)
  (export "addTwo" (func 0)))
  
------------------------------------------------------------------
```
Now, let's deal with the tedious binry format. There is also an useful tool called `xxd` in linux to dump hexadecimal byte from file. `wabt` also provide `wasm-objdump` which is just like the familiar `objdump` to decompose the file into multiple sections.

1. According to the [offical spec](https://webassembly.github.io/spec/core/binary/modules.html#binary-module), `wasm` binary format start with a 4-byte magic number `0x00 0x61 0x73 0x6D` and it's 4-byte version which is `0x01 0x00  0x00 0x00`.
2. The following byte `0x01` shows section id and `0x01` means `type section`. The spec also offers a [table](https://webassembly.github.io/spec/core/binary/modules.html#sections) to check the id belong to each section. `0x07` says the section is encoded into 7-byte length and the content is `0x01 0x60 0x02 0x7f 0x7f 0x01 0x7f`. 
```
---------------- Hexadecimal byte representation -----------------
$ xxd addTwo.wasm
00000000: 0061 736d 0100 0000 0107 0160 027f 7f01  .asm.......`....
00000010: 7f03 0201 0007 0a01 0661 6464 5477 6f00  .........addTwo.
00000020: 000a 0901 0700 2000 2001 6a0b            ...... . .j.

$ wasm-objdump -s addTwo.wasm
addTwo.wasm:	file format wasm 0x1

Contents of section Type:
000000a: 0160 027f 7f01 7f                        .`.....

Contents of section Function:
0000013: 0100                                     ..

Contents of section Export:
0000017: 0106 6164 6454 776f 0000                 ..addTwo..

Contents of section Code:
0000023: 0107 0020 0020 016a 0b                   ... . .j.
-------------------------------------------------------------------
```
## Binary Instrumentation
### Goal
To measure the performance, the easiest way is to set a timer when entering target function and stop it while exiting. For demonstration, we will deserialize our previous `addTwo.wasm`. Here, we rename it as `template.wasm` and instrument the `enterFunc` function at the start of `addTwo` function. Although our intention seems really easy, there are still several task we need to conquer.
* **Implicit function dependencies**
The injected functions possibly rely on their dependencies to fully functional. When trying to deploy our WebAssembly application on system, it would become even more complex. For example, the `printf` function requires imported `wasi-libc` syscall implementation. We need to deal with the dependent graph.
* **Memory manipulation**
To do even simplest thing requires memory accessing. We need to develop an independent memory model and integrate it to existing memory management system perfectly.
```C
// template.c
#include <stdio.h>
int addTwo(int a, int b) {
    return a + b;
}
int main() {
    addTwo(1, 2);
}

// instrument.c
void enterFunc() {
    printf("entering function");
}
```
### Hook instrumentation and Name Section
#### target identification
To complete our task, first step is to identify where our target is. The `wasm` file doesn't encode function name by default. To figure out which function is our target we need to add debug info during compilation by specify `-g2` flag. With `-g2` flag, the compiler would encode all the function name in custom section as below.
```
$ emcc -O1 -g2 -s WASM=1 -s  ONLY_MY_CODE=0 instrument.c -o instrument.wasm
$ emcc -O1 -g2 -s WASM=1 -s  ONLY_MY_CODE=0 template.c -o template.wasm

+--------------------------------------+
| Custom Name section in template.wasm |
+--------------------------------------+
Contents of section Custom:
0000081: 046e 616d 6501 1102 0007 5f61 6464 5477  .name....._addTw
0000091: 6f01 055f 6d61 696e                      o.._main

+----------------------------------------+
| Custom Name section in instrument.wasm |
+----------------------------------------+
Contents of section Custom:
0004b46: 046e 616d 6501 c405 3700 0561 626f 7274  .name...7..abort
0004b56: 010b 5f5f 5f73 6574 4572 724e 6f02 0d5f  ..___setErrNo.._
0004b66: 5f5f 7379 7363 616c 6c31 3430 030d 5f5f  __syscall140..__
0004b76: 5f73 7973 6361 6c6c 3134 3604 0c5f 5f5f  _syscall146..___
0004b86: 7379 7363 616c 6c35 3405 0b5f 5f5f 7379  syscall54..___sy
0004b96: 7363 616c 6c36 0619 5f65 6d73 6372 6970  scall6.._emscrip
0004ba6: 7465 6e5f 6765 745f 6865 6170 5f73 697a  ten_get_heap_siz
0004bb6: 6507 165f 656d 7363 7269 7074 656e 5f6d  e.._emscripten_m
0004bc6: 656d 6370 795f 6269 6708 175f 656d 7363  emcpy_big.._emsc
0004bd6: 7269 7074 656e 5f72 6573 697a 655f 6865  ripten_resize_he
0004be6: 6170 0917 6162 6f72 744f 6e43 616e 6e6f  ap..abortOnCanno
0004bf6: 7447 726f 774d 656d 6f72 790a 0a73 7461  tGrowMemory..sta
0004c06: 636b 416c 6c6f 630b 0973 7461 636b 5361  ckAlloc..stackSa
0004c16: 7665 0c0c 7374 6163 6b52 6573 746f 7265  ve..stackRestore
0004c26: 0d13 6573 7461 626c 6973 6853 7461 636b  ..establishStack
0004c36: 5370 6163 650e 0a5f 656e 7465 7246 756e  Space.._enterFun
0004c46: 630f 075f 6d61 6c6c 6f63 1005 5f66 7265  c.._malloc.._fre
0004c56: 6511 0e5f 5f5f 7374 6469 6f5f 636c 6f73  e..___stdio_clos
0004c66: 6512 0e5f 5f5f 7374 6469 6f5f 7772 6974  e..___stdio_writ
0004c76: 6513 0d5f 5f5f 7374 6469 6f5f 7365 656b  e..___stdio_seek
0004c86: 140e 5f5f 5f73 7973 6361 6c6c 5f72 6574  ..___syscall_ret
0004c96: 1511 5f5f 5f65 7272 6e6f 5f6c 6f63 6174  ..___errno_locat
0004ca6: 696f 6e16 065f 6475 6d6d 7917 0f5f 5f5f  ion.._dummy..___
0004cb6: 7374 646f 7574 5f77 7269 7465 1808 5f69  stdout_write.._i
0004cc6: 7364 6967 6974 190d 5f70 7468 7265 6164  sdigit.._pthread
0004cd6: 5f73 656c 661a 0d5f 5f5f 756e 6c6f 636b  _self..___unlock
0004ce6: 6669 6c65 1b0b 5f5f 5f6c 6f63 6b66 696c  file..___lockfil
0004cf6: 651c 0a5f 5f5f 746f 7772 6974 651d 0a5f  e..___towrite.._
0004d06: 5f5f 6677 7269 7465 781e 075f 6d65 6d63  __fwritex.._memc
0004d16: 6872 1f09 5f76 6670 7269 6e74 6620 0c5f  hr.._vfprintf ._
0004d26: 7072 696e 7466 5f63 6f72 6521 085f 6f75  printf_core!._ou
0004d36: 745f 3635 3522 0b5f 6765 7469 6e74 5f36  t_655"._getint_6
0004d46: 3536 230c 5f70 6f70 5f61 7267 5f36 3538  56#._pop_arg_658
0004d56: 2406 5f66 6d74 5f78 2506 5f66 6d74 5f6f  $._fmt_x%._fmt_o
0004d66: 2606 5f66 6d74 5f75 2708 5f70 6164 5f36  &._fmt_u'._pad_6
0004d76: 3631 2807 5f77 6374 6f6d 6229 075f 666d  61(._wctomb)._fm
0004d86: 745f 6670 2a12 5f5f 5f44 4f55 424c 455f  t_fp*.___DOUBLE_
0004d96: 4249 5453 5f36 3632 2b07 5f66 7265 7870  BITS_662+._frexp
0004da6: 6c2c 065f 6672 6578 702d 085f 7763 7274  l,._frexp-._wcrt
0004db6: 6f6d 622e 135f 5f5f 7074 6872 6561 645f  omb..___pthread_
0004dc6: 7365 6c66 5f38 3838 2f07 5f70 7269 6e74  self_888/._print
0004dd6: 6630 075f 6d65 6d63 7079 3107 5f6d 656d  f0._memcpy1._mem
0004de6: 7365 7432 055f 7362 726b 330a 6479 6e43  set2._sbrk3.dynC
0004df6: 616c 6c5f 6969 340c 6479 6e43 616c 6c5f  all_ii4.dynCall_
0004e06: 6969 6969 3502 6230 3602 6231            iiii5.b06.b1

+---------------------------------+
| Import section in template.wasm |
+---------------------------------+
Contents of section Import:
0000064: 0903 656e 7605 6162 6f72 7400 0203 656e  ..env.abort...en
0000074: 760d 5f5f 5f73 7973 6361 6c6c 3134 3600  v.___syscall146.
0000084: 0303 656e 7616 5f65 6d73 6372 6970 7465  ..env._emscripte
0000094: 6e5f 6d65 6d63 7079 5f62 6967 0000 0365  n_memcpy_big...e
00000a4: 6e76 0b5f 5f5f 7379 7363 616c 6c36 0003  nv.___syscall6..
00000b4: 0365 6e76 0c5f 5f5f 7379 7363 616c 6c35  .env.___syscall5
00000c4: 3400 0303 656e 760d 5f5f 5f73 7973 6361  4...env.___sysca
00000d4: 6c6c 3134 3000 0303 656e 760c 5f5f 7461  ll140...env.__ta
00000e4: 626c 655f 6261 7365 037f 0003 656e 7606  ble_base....env.
00000f4: 6d65 6d6f 7279 0201 8002 8002 0365 6e76  memory.......env
0000104: 0574 6162 6c65 0170 0106 06              .table.p...

```
As you can see, even a simple `printf` function requires suprisingly many dependencies. We even don't know where they come from? In WebAssembly, they eithter imported from `import section` or are defined in `function section`. Thus, we deserialize the `import section` and find out from `env.abort` to `env.abort` come from imported section. However, those imported components are not all function. Luckily there is an useful crate [parity-wasm](https://github.com/paritytech/parity-wasm) to parse all function out.

```Rust
fn search_metadata(func_name: String, deserialize_mod: &parity_wasm::elements::Module) {
    // Count how many function are imported
    let mut import_num: usize = 0;
    match deserialize_mod.import_section() {
        Some(sec) => {
            import_num = sec.functions();
        },
        _ => {},
    }
    let names_section = deserialize_mod.names_section();
    let name_map = names_section.unwrap().functions().unwrap().names();
    // Get the idex of our target function 
    let func_id = decode_func_id(name_map, &func_name);
    let code_section = deserialize_mod.code_section().unwrap();
    // Subtract the import_num from index to skip the imported function. 
    identify_dependency(code_section.bodies()[func_id.unwrap() as usize - import_num].code(), name_map);
}
```
Before we really instrument some binary bytecode in our `wasm` code, we still have several things to deal with.
1. Maintain the verification of binary file such as the length of `code section`, type in the `type section`, data in `data section` ....
2. If the injected function involve memory accessment, we need to isolate a part of memory for it.

#### instrument a little `call` op
As usual, let's consider to do the easiest job first, and leave the difficult format maintainance behind. How about integrating our callee aka instrumented function `enterFunc` and caller aka `addTwo` function in same file first and call it manually. In this example, we don't have to worry about all the details I mention above. All we have to concern is to call `enterFunc` from bytecode.

```C
// call_manually.c
#include <stdio.h>
void enterFunc() {
    printf("entering function\n");   
}

int addTwo(int a, int b) {
    // We try to call enterFunc here manually.
    // enterFunc();
    return a + b;
}
int main() {
    addTwo(1, 2);
}
```
Again, looking up specification is the first step to solve the task and refer to binary difference in `wasm` file.
```

```


## Static and Dynamic linking
Preset profiler can be categorized into two classes.

1. Static instrumentation
2. Dynamic linking

Briefly, what we already done previously is static instrumentation. "Static" means we directly manipulate our target and inject extra operations such as "call", "nop" and so on. Moreover, we are able to inject a whole function or code segment into binary format as soon as we deal all the corresponding adjustment.

On the contrary, "dynamic" means we interfere the application behavior indirectly. WebAssembly already offers "dynamic linking" feature in MVP and current runtime implement it as well. With dynamic linking, we enjoy several advantages.

1. The size of target application remains the same.
2. The modified program acts much similar to original one.
3. With fewer code injection, we won't suffer from its side effect like memory manipulation, recording the state of stack frame and so on.
4. Profile those functions which is not defined by our own.

So how do we exactly implement dynamic linking to fulfill our measurement? Just like "linker interposition" in Linux, all we have to do is to replace the customize function in the other module. Let's say we have function "foo" in `a.c` and we try to replace the callee to the function "foo_modified" in `b.c`.

```C
# a.c -> a.wasm
void foo() {
    printf("this is original foo function");
}
int main() {
    foo();
}

# b.c -> b.wasm
void foo_modified() {
    printf("this is modified foo function");
}
```

### Issue
- [x] automatic inlining
> use compiler attribute `void __attribute__((optnone)) foo() {}`
> [color=#2dbf0d]
- [ ] filter out dependency from `import` section
- [ ] memory model
- [ ] restore stack frame state
- [ ] linker
- [ ] Dead code elimination
- [ ] Where to inject
## Relative Repo
* [wac](https://github.com/kanaka/wac/tree/master/examples_wast)
* [wasm-interp](https://github.com/WebAssembly/wabt) in wabt
* [wasmparser](https://docs.rs/wasmparser/0.29.2/wasmparser/)
* [WasmExplorer](http://mbebenita.github.io/WasmExplorer/)
* [wasmtime](https://github.com/CraneStation/wasmtime)
* [cranelift-wasm](https://github.com/CraneStation/cranelift/tree/master/cranelift-wasm)
## Runtime
* execute hexadecimal byte code in memory [ref](https://stackoverflow.com/questions/18476002/execute-binary-machine-code-from-c)
* [Compiling C to WebAssembly using clang/LLVM and WASI](https://00f.net/2019/04/07/compiling-to-webassembly-with-llvm-and-clang/)
* [tool convention/linking](https://github.com/WebAssembly/tool-conventions/blob/master/Linking.md#linking-metadata-section)
### wasm layout
* wasm [magic U32](https://github.com/WebAssembly/wabt/blob/master/src/binary-reader.cc#L2316)
* wasm [version U32](https://github.com/WebAssembly/wabt/blob/master/src/binary-reader.cc#L2319) 
* 
### SIMD implementation
* wasm issue [fix-width SIMD](https://github.com/WebAssembly/proposals/issues/1)
* [SIMD in WebAssembly – tales from the bleeding edge](https://brionv.com/log/2019/03/03/simd-in-webassembly-tales-from-the-bleeding-edge/)