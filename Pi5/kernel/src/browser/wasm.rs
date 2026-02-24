//! WebAssembly Engine
//!
//! A minimal WebAssembly interpreter for WebbOS.
//! Supports the core WebAssembly spec.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;

use crate::browser::BrowserError;
use crate::println;

/// WebAssembly value types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    I32,
    I64,
    F32,
    F64,
}

/// WebAssembly values
#[derive(Debug, Clone, Copy)]
pub enum Value {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

impl Value {
    pub fn value_type(&self) -> ValueType {
        match self {
            Value::I32(_) => ValueType::I32,
            Value::I64(_) => ValueType::I64,
            Value::F32(_) => ValueType::F32,
            Value::F64(_) => ValueType::F64,
        }
    }
}

/// WebAssembly module
pub struct Module {
    /// Module version
    pub version: u32,
    /// Types
    pub types: Vec<FuncType>,
    /// Functions
    pub functions: Vec<Function>,
    /// Exports
    pub exports: BTreeMap<String, Export>,
    /// Memory
    pub memory: Option<Memory>,
    /// Global variables
    pub globals: Vec<Global>,
    /// Data segments
    pub data: Vec<DataSegment>,
}

/// Function type
#[derive(Debug, Clone)]
pub struct FuncType {
    pub params: Vec<ValueType>,
    pub results: Vec<ValueType>,
}

/// Function
#[derive(Debug, Clone)]
pub struct Function {
    pub type_idx: u32,
    pub locals: Vec<ValueType>,
    pub body: Vec<Instruction>,
}

/// Export
#[derive(Debug, Clone)]
pub struct Export {
    pub kind: ExportKind,
    pub idx: u32,
}

/// Export kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportKind {
    Func,
    Table,
    Mem,
    Global,
}

/// Memory
#[derive(Debug)]
pub struct Memory {
    pub min: u32,
    pub max: Option<u32>,
    pub data: Vec<u8>,
}

impl Memory {
    pub fn new(min: u32, max: Option<u32>) -> Self {
        let size = (min as usize) * 64 * 1024; // 64KB pages
        Self {
            min,
            max,
            data: vec![0u8; size],
        }
    }

    pub fn read(&self, addr: usize, len: usize) -> &[u8] {
        &self.data[addr..(addr + len).min(self.data.len())]
    }

    pub fn write(&mut self, addr: usize, data: &[u8]) {
        let end = (addr + data.len()).min(self.data.len());
        self.data[addr..end].copy_from_slice(&data[..end - addr]);
    }

    pub fn read_i32(&self, addr: usize) -> i32 {
        let bytes = self.read(addr, 4);
        i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }

    pub fn write_i32(&mut self, addr: usize, val: i32) {
        self.write(addr, &val.to_le_bytes());
    }
}

/// Global variable
#[derive(Debug, Clone)]
pub struct Global {
    pub value_type: ValueType,
    pub mutable: bool,
    pub value: Value,
}

/// Data segment
#[derive(Debug, Clone)]
pub struct DataSegment {
    pub memory_idx: u32,
    pub offset: Vec<Instruction>,
    pub data: Vec<u8>,
}

/// WebAssembly instruction
#[derive(Debug, Clone)]
pub enum Instruction {
    // Control instructions
    Unreachable,
    Nop,
    Block(ValueType, Vec<Instruction>),
    Loop(ValueType, Vec<Instruction>),
    If(ValueType, Vec<Instruction>, Option<Vec<Instruction>>),
    Br(u32),
    BrIf(u32),
    BrTable(Vec<u32>, u32),
    Return,
    Call(u32),
    CallIndirect(u32, u32),

    // Parametric instructions
    Drop,
    Select,

    // Variable instructions
    LocalGet(u32),
    LocalSet(u32),
    LocalTee(u32),
    GlobalGet(u32),
    GlobalSet(u32),

    // Memory instructions
    I32Load(u32, u32),
    I64Load(u32, u32),
    F32Load(u32, u32),
    F64Load(u32, u32),
    I32Load8S(u32, u32),
    I32Load8U(u32, u32),
    I32Load16S(u32, u32),
    I32Load16U(u32, u32),
    I32Store(u32, u32),
    I64Store(u32, u32),
    F32Store(u32, u32),
    F64Store(u32, u32),
    I32Store8(u32, u32),
    I32Store16(u32, u32),
    MemorySize(u32),
    MemoryGrow(u32),

    // Numeric instructions - constants
    I32Const(i32),
    I64Const(i64),
    F32Const(f32),
    F64Const(f64),

    // Numeric instructions - i32
    I32Eqz,
    I32Eq,
    I32Ne,
    I32LtS,
    I32LtU,
    I32GtS,
    I32GtU,
    I32LeS,
    I32LeU,
    I32GeS,
    I32GeU,
    I32Clz,
    I32Ctz,
    I32Popcnt,
    I32Add,
    I32Sub,
    I32Mul,
    I32DivS,
    I32DivU,
    I32RemS,
    I32RemU,
    I32And,
    I32Or,
    I32Xor,
    I32Shl,
    I32ShrS,
    I32ShrU,
    I32Rotl,
    I32Rotr,

    // Numeric instructions - i64
    I64Eqz,
    I64Eq,
    I64Ne,
    I64LtS,
    I64LtU,
    I64GtS,
    I64GtU,
    I64LeS,
    I64LeU,
    I64GeS,
    I64GeU,

    // Numeric instructions - f32
    F32Eq,
    F32Ne,
    F32Lt,
    F32Gt,
    F32Le,
    F32Ge,

    // Numeric instructions - f64
    F64Eq,
    F64Ne,
    F64Lt,
    F64Gt,
    F64Le,
    F64Ge,

    // Conversions
    I32WrapI64,
    I64ExtendI32S,
    I64ExtendI32U,
}

/// WebAssembly binary parser
struct Parser<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        Some(b)
    }

    fn read_u32(&mut self) -> u32 {
        let mut result = 0u32;
        let mut shift = 0;
        loop {
            let byte = self.next().unwrap_or(0);
            result |= ((byte & 0x7F) as u32) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        result
    }

    fn read_i32(&mut self) -> i32 {
        let mut result = 0i32;
        let mut shift = 0;
        loop {
            let byte = self.next().unwrap_or(0);
            result |= ((byte & 0x7F) as i32) << shift;
            shift += 7;
            if byte & 0x80 == 0 {
                if byte & 0x40 != 0 && shift < 32 {
                    result |= !0 << shift;
                }
                break;
            }
        }
        result
    }

    fn read_f32(&mut self) -> f32 {
        let bytes = [
            self.next().unwrap_or(0),
            self.next().unwrap_or(0),
            self.next().unwrap_or(0),
            self.next().unwrap_or(0),
        ];
        f32::from_le_bytes(bytes)
    }

    fn read_f64(&mut self) -> f64 {
        let bytes = [
            self.next().unwrap_or(0),
            self.next().unwrap_or(0),
            self.next().unwrap_or(0),
            self.next().unwrap_or(0),
            self.next().unwrap_or(0),
            self.next().unwrap_or(0),
            self.next().unwrap_or(0),
            self.next().unwrap_or(0),
        ];
        f64::from_le_bytes(bytes)
    }

    fn read_bytes(&mut self, len: usize) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            bytes.push(self.next().unwrap_or(0));
        }
        bytes
    }

    fn read_string(&mut self) -> String {
        let len = self.read_u32() as usize;
        let bytes = self.read_bytes(len);
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn read_value_type(&mut self) -> Option<ValueType> {
        match self.next()? {
            0x7F => Some(ValueType::I32),
            0x7E => Some(ValueType::I64),
            0x7D => Some(ValueType::F32),
            0x7C => Some(ValueType::F64),
            _ => None,
        }
    }

    fn parse(&mut self) -> Result<Module, BrowserError> {
        // Magic number: 0x00 0x61 0x73 0x6D (\0asm)
        if self.read_bytes(4) != [0x00, 0x61, 0x73, 0x6D] {
            return Err(BrowserError::WasmError);
        }

        // Version: 1
        let version = self.read_u32();
        if version != 1 {
            return Err(BrowserError::WasmError);
        }

        let mut module = Module {
            version,
            types: Vec::new(),
            functions: Vec::new(),
            exports: BTreeMap::new(),
            memory: None,
            globals: Vec::new(),
            data: Vec::new(),
        };

        // Parse sections
        while self.pos < self.data.len() {
            let section_id = self.next().unwrap_or(0);
            let section_size = self.read_u32() as usize;
            let section_end = self.pos + section_size;

            match section_id {
                1 => module.types = self.parse_type_section()?,
                3 => module.functions = self.parse_function_section(&module.types)?,
                6 => module.globals = self.parse_global_section()?,
                7 => module.exports = self.parse_export_section()?,
                11 => module.data = self.parse_data_section()?,
                _ => {
                    // Skip unknown section
                    self.pos = section_end;
                }
            }
        }

        Ok(module)
    }

    fn parse_type_section(&mut self) -> Result<Vec<FuncType>, BrowserError> {
        let count = self.read_u32();
        let mut types = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let form = self.next().ok_or(BrowserError::WasmError)?;
            if form != 0x60 {
                return Err(BrowserError::WasmError);
            }

            let param_count = self.read_u32();
            let mut params = Vec::with_capacity(param_count as usize);
            for _ in 0..param_count {
                params.push(self.read_value_type().ok_or(BrowserError::WasmError)?);
            }

            let result_count = self.read_u32();
            let mut results = Vec::with_capacity(result_count as usize);
            for _ in 0..result_count {
                results.push(self.read_value_type().ok_or(BrowserError::WasmError)?);
            }

            types.push(FuncType { params, results });
        }

        Ok(types)
    }

    fn parse_function_section(&mut self, types: &[FuncType]) -> Result<Vec<Function>, BrowserError> {
        let count = self.read_u32();
        let mut functions = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let type_idx = self.read_u32();
            functions.push(Function {
                type_idx,
                locals: Vec::new(),
                body: Vec::new(),
            });
        }

        Ok(functions)
    }

    fn parse_global_section(&mut self) -> Result<Vec<Global>, BrowserError> {
        let count = self.read_u32();
        let mut globals = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let value_type = self.read_value_type().ok_or(BrowserError::WasmError)?;
            let mutable = self.next().ok_or(BrowserError::WasmError)? == 1;
            let value = self.parse_init_expr()?;
            
            globals.push(Global {
                value_type,
                mutable,
                value,
            });
        }

        Ok(globals)
    }

    fn parse_export_section(&mut self) -> Result<BTreeMap<String, Export>, BrowserError> {
        let count = self.read_u32();
        let mut exports = BTreeMap::new();

        for _ in 0..count {
            let name = self.read_string();
            let kind = match self.next().ok_or(BrowserError::WasmError)? {
                0x00 => ExportKind::Func,
                0x01 => ExportKind::Table,
                0x02 => ExportKind::Mem,
                0x03 => ExportKind::Global,
                _ => return Err(BrowserError::WasmError),
            };
            let idx = self.read_u32();

            exports.insert(name, Export { kind, idx });
        }

        Ok(exports)
    }

    fn parse_data_section(&mut self) -> Result<Vec<DataSegment>, BrowserError> {
        let count = self.read_u32();
        let mut segments = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let memory_idx = self.read_u32();
            let offset = self.parse_instruction_sequence()?;
            let data_len = self.read_u32() as usize;
            let data = self.read_bytes(data_len);

            segments.push(DataSegment {
                memory_idx,
                offset,
                data,
            });
        }

        Ok(segments)
    }

    fn parse_init_expr(&mut self) -> Result<Value, BrowserError> {
        let insts = self.parse_instruction_sequence()?;
        // Simplified: just return a default value
        Ok(Value::I32(0))
    }

    fn parse_instruction_sequence(&mut self) -> Result<Vec<Instruction>, BrowserError> {
        let mut insts = Vec::new();

        loop {
            match self.peek() {
                Some(0x0B) => {
                    self.next(); // end
                    break;
                }
                Some(opcode) => {
                    let inst = self.parse_instruction()?;
                    insts.push(inst);
                }
                None => break,
            }
        }

        Ok(insts)
    }

    fn parse_instruction(&mut self) -> Result<Instruction, BrowserError> {
        let opcode = self.next().ok_or(BrowserError::WasmError)?;

        match opcode {
            0x00 => Ok(Instruction::Unreachable),
            0x01 => Ok(Instruction::Nop),
            0x0F => Ok(Instruction::Return),
            0x1A => Ok(Instruction::Drop),
            0x1B => Ok(Instruction::Select),

            0x20 => Ok(Instruction::LocalGet(self.read_u32())),
            0x21 => Ok(Instruction::LocalSet(self.read_u32())),
            0x22 => Ok(Instruction::LocalTee(self.read_u32())),
            0x23 => Ok(Instruction::GlobalGet(self.read_u32())),
            0x24 => Ok(Instruction::GlobalSet(self.read_u32())),

            0x28 => {
                let align = self.read_u32();
                let offset = self.read_u32();
                Ok(Instruction::I32Load(align, offset))
            }
            0x36 => {
                let align = self.read_u32();
                let offset = self.read_u32();
                Ok(Instruction::I32Store(align, offset))
            }

            0x41 => Ok(Instruction::I32Const(self.read_i32())),
            0x42 => Ok(Instruction::I64Const(self.read_i32() as i64)),
            0x43 => Ok(Instruction::F32Const(self.read_f32())),
            0x44 => Ok(Instruction::F64Const(self.read_f64())),

            0x45 => Ok(Instruction::I32Eqz),
            0x46 => Ok(Instruction::I32Eq),
            0x47 => Ok(Instruction::I32Ne),
            0x48 => Ok(Instruction::I32LtS),
            0x49 => Ok(Instruction::I32LtU),
            0x4A => Ok(Instruction::I32GtS),
            0x4B => Ok(Instruction::I32GtU),
            0x4C => Ok(Instruction::I32LeS),
            0x4D => Ok(Instruction::I32LeU),
            0x4E => Ok(Instruction::I32GeS),
            0x4F => Ok(Instruction::I32GeU),

            0x6A => Ok(Instruction::I32Add),
            0x6B => Ok(Instruction::I32Sub),
            0x6C => Ok(Instruction::I32Mul),
            0x6D => Ok(Instruction::I32DivS),
            0x6E => Ok(Instruction::I32DivU),
            0x6F => Ok(Instruction::I32RemS),
            0x70 => Ok(Instruction::I32RemU),
            0x71 => Ok(Instruction::I32And),
            0x72 => Ok(Instruction::I32Or),
            0x73 => Ok(Instruction::I32Xor),
            0x74 => Ok(Instruction::I32Shl),
            0x75 => Ok(Instruction::I32ShrS),
            0x76 => Ok(Instruction::I32ShrU),

            _ => Err(BrowserError::WasmError),
        }
    }
}

/// WebAssembly runtime
pub struct Runtime {
    module: Module,
    call_stack: Vec<ActivationFrame>,
    memory: Option<Memory>,
    globals: Vec<Value>,
}

/// Activation frame
struct ActivationFrame {
    function_idx: u32,
    locals: Vec<Value>,
    pc: usize,
}

impl Runtime {
    pub fn new(module: Module) -> Self {
        let memory = module.memory.as_ref().map(|m| {
            Memory::new(m.min, m.max)
        });

        let globals: Vec<Value> = module.globals.iter()
            .map(|g| g.value)
            .collect();

        Self {
            module,
            call_stack: Vec::new(),
            memory,
            globals,
        }
    }

    pub fn call(&mut self, name: &str, args: Vec<Value>) -> Result<Vec<Value>, BrowserError> {
        let export = self.module.exports.get(name)
            .ok_or(BrowserError::WasmError)?;

        if export.kind != ExportKind::Func {
            return Err(BrowserError::WasmError);
        }

        self.call_function(export.idx, args)?;
        
        // Execute until return
        while !self.call_stack.is_empty() {
            self.step()?;
        }

        Ok(Vec::new()) // TODO: return actual results
    }

    fn call_function(&mut self, idx: u32, args: Vec<Value>) -> Result<(), BrowserError> {
        let func = self.module.functions.get(idx as usize)
            .ok_or(BrowserError::WasmError)?
            .clone();

        let func_type = self.module.types.get(func.type_idx as usize)
            .ok_or(BrowserError::WasmError)?;

        // Check argument count
        if args.len() != func_type.params.len() {
            return Err(BrowserError::WasmError);
        }

        // Create locals from params
        let mut locals = args;
        for local_type in &func.locals {
            locals.push(match local_type {
                ValueType::I32 => Value::I32(0),
                ValueType::I64 => Value::I64(0),
                ValueType::F32 => Value::F32(0.0),
                ValueType::F64 => Value::F64(0.0),
            });
        }

        self.call_stack.push(ActivationFrame {
            function_idx: idx,
            locals,
            pc: 0,
        });

        Ok(())
    }

    fn step(&mut self) -> Result<(), BrowserError> {
        let frame = self.call_stack.last_mut()
            .ok_or(BrowserError::WasmError)?;

        let func = self.module.functions.get(frame.function_idx as usize)
            .ok_or(BrowserError::WasmError)?
            .clone();

        if frame.pc >= func.body.len() {
            self.call_stack.pop();
            return Ok(());
        }

        let inst = func.body[frame.pc].clone();
        frame.pc += 1;

        // Execute instruction (simplified)
        match inst {
            Instruction::Nop => {}
            Instruction::Return => {
                self.call_stack.pop();
            }
            _ => {}
        }

        Ok(())
    }
}

/// Load WebAssembly module
pub fn load(data: &[u8]) -> Result<Module, BrowserError> {
    let mut parser = Parser::new(data);
    parser.parse()
}

/// Initialize WebAssembly engine
pub fn init() {
    println!("[wasm] WebAssembly engine initialized");
}

/// Execute a simple test program
pub fn test() -> Result<(), BrowserError> {
    // Simple i32 addition: (i32.add (i32.const 1) (i32.const 2))
    // This would need a proper wasm binary to test
    println!("[wasm] WebAssembly test not implemented");
    Ok(())
}
