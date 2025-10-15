//! SPIR-V IR emitter for GPU instructions

pub struct Emitter {
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub opcode: String,
    pub operands: Vec<u64>,
}

impl Emitter {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
        }
    }
    
    pub fn emit(&mut self, opcode: &str, operands: Vec<u64>) {
        self.instructions.push(Instruction {
            opcode: opcode.to_string(),
            operands,
        });
    }
    
    /// Clear all emitted instructions
    pub fn clear(&mut self) {
        self.instructions.clear();
    }
    
    /// Get number of emitted instructions
    pub fn len(&self) -> usize {
        self.instructions.len()
    }
    
    /// Check if no instructions have been emitted
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }
}

impl Default for Emitter {
    fn default() -> Self {
        Self::new()
    }
}