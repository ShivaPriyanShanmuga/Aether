//! AIR data structures: modules, functions, blocks, instructions, and values.

use aether_source::Span;

/// An AIR type. Only a 64-bit integer exists today; more types arrive with the
/// type system.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Type {
    /// The 64-bit signed integer type, `int`.
    Int,
}

impl Type {
    /// The type's textual name (e.g. `"int"`).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Type::Int => "int",
        }
    }
}

/// A binary arithmetic operator.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinaryOp {
    /// Integer addition.
    Add,
    /// Integer subtraction.
    Sub,
    /// Integer multiplication.
    Mul,
    /// Integer division.
    Div,
}

impl BinaryOp {
    /// The mnemonic used in AIR's textual form (e.g. `"add"`).
    #[must_use]
    pub fn mnemonic(self) -> &'static str {
        match self {
            BinaryOp::Add => "add",
            BinaryOp::Sub => "sub",
            BinaryOp::Mul => "mul",
            BinaryOp::Div => "div",
        }
    }
}

/// A unary operator.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UnaryOp {
    /// Integer negation.
    Neg,
}

impl UnaryOp {
    /// The mnemonic used in AIR's textual form (e.g. `"neg"`).
    #[must_use]
    pub fn mnemonic(self) -> &'static str {
        match self {
            UnaryOp::Neg => "neg",
        }
    }
}

/// A reference to an SSA value: the result of an instruction.
///
/// A `Value` is an index into its [`Function`]'s instruction arena.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Value(u32);

impl Value {
    /// The index this value refers to.
    #[must_use]
    pub fn index(self) -> usize {
        self.0 as usize
    }

    pub(crate) fn from_index(index: usize) -> Value {
        Value(index as u32)
    }
}

/// A reference to a basic block within a [`Function`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Block(u32);

impl Block {
    /// The index this block refers to.
    #[must_use]
    pub fn index(self) -> usize {
        self.0 as usize
    }

    pub(crate) fn from_index(index: usize) -> Block {
        Block(index as u32)
    }
}

/// The operation an instruction performs. Payload operands are [`Value`]s.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InstData {
    /// An integer constant.
    IConst(i64),
    /// A unary operation on one value.
    Unary {
        /// The operator.
        op: UnaryOp,
        /// The operand.
        operand: Value,
    },
    /// A binary operation on two values.
    Binary {
        /// The operator.
        op: BinaryOp,
        /// The left operand.
        lhs: Value,
        /// The right operand.
        rhs: Value,
    },
}

/// An instruction: its operation, result type, and originating source span.
#[derive(Clone, Copy, Debug)]
pub struct Inst {
    /// The operation performed.
    pub data: InstData,
    /// The type of the value this instruction produces.
    pub ty: Type,
    /// The source location this instruction was lowered from.
    pub span: Span,
}

/// How a basic block ends and transfers control.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Terminator {
    /// Return a value from the function.
    Ret(Value),
}

/// The contents of a basic block: an ordered list of the values it computes,
/// followed by a terminator.
#[derive(Clone, Debug, Default)]
pub struct BlockData {
    /// The values computed in this block, in execution order.
    pub body: Vec<Value>,
    /// How the block ends. `None` only while the block is being built; a
    /// well-formed block always has a terminator (checked by [`verify`]).
    pub terminator: Option<Terminator>,
}

/// A function: a name, a return type, and a control-flow graph of basic blocks.
#[derive(Clone, Debug)]
pub struct Function {
    /// The function's name.
    pub name: String,
    /// The function's return type.
    pub return_type: Type,
    /// Instruction arena, addressed by [`Value`].
    insts: Vec<Inst>,
    /// Basic-block arena, addressed by [`Block`].
    blocks: Vec<BlockData>,
    /// The entry block, always present.
    entry: Block,
}

impl Function {
    /// Create a function with the given name and return type. An empty entry
    /// block is created automatically.
    #[must_use]
    pub fn new(name: impl Into<String>, return_type: Type) -> Function {
        Function {
            name: name.into(),
            return_type,
            insts: Vec::new(),
            blocks: vec![BlockData::default()],
            entry: Block::from_index(0),
        }
    }

    /// The entry block.
    #[must_use]
    pub fn entry(&self) -> Block {
        self.entry
    }

    /// Append a new, empty basic block and return its id.
    pub fn append_block(&mut self) -> Block {
        let id = Block::from_index(self.blocks.len());
        self.blocks.push(BlockData::default());
        id
    }

    /// Append an instruction to `block`, returning the [`Value`] it defines.
    pub fn push_inst(&mut self, block: Block, data: InstData, ty: Type, span: Span) -> Value {
        let value = Value::from_index(self.insts.len());
        self.insts.push(Inst { data, ty, span });
        self.blocks[block.index()].body.push(value);
        value
    }

    /// Set the terminator of `block`.
    pub fn set_terminator(&mut self, block: Block, terminator: Terminator) {
        self.blocks[block.index()].terminator = Some(terminator);
    }

    /// The instruction that defines `value`.
    #[must_use]
    pub fn inst(&self, value: Value) -> &Inst {
        &self.insts[value.index()]
    }

    /// The type of `value`.
    #[must_use]
    pub fn value_type(&self, value: Value) -> Type {
        self.insts[value.index()].ty
    }

    /// The number of values (instructions) defined in this function.
    #[must_use]
    pub fn value_count(&self) -> usize {
        self.insts.len()
    }

    /// The contents of `block`.
    #[must_use]
    pub fn block(&self, block: Block) -> &BlockData {
        &self.blocks[block.index()]
    }

    /// All basic blocks, in id order (id == position).
    #[must_use]
    pub fn blocks(&self) -> &[BlockData] {
        &self.blocks
    }
}

/// A translation unit: a collection of functions.
#[derive(Clone, Debug, Default)]
pub struct Module {
    functions: Vec<Function>,
}

impl Module {
    /// Create an empty module.
    #[must_use]
    pub fn new() -> Module {
        Module::default()
    }

    /// Append a function to the module.
    pub fn add_function(&mut self, function: Function) {
        self.functions.push(function);
    }

    /// The module's functions.
    #[must_use]
    pub fn functions(&self) -> &[Function] {
        &self.functions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_source::{BytePos, FileId, SourceMap};

    fn dummy_span() -> Span {
        let mut map = SourceMap::new();
        let file: FileId = map.add_file("t.ae", "");
        Span::new(file, BytePos(0), BytePos(0))
    }

    #[test]
    fn build_a_function() {
        let span = dummy_span();
        let mut f = Function::new("main", Type::Int);
        let entry = f.entry();
        let a = f.push_inst(entry, InstData::IConst(1), Type::Int, span);
        let b = f.push_inst(entry, InstData::IConst(2), Type::Int, span);
        let sum = f.push_inst(
            entry,
            InstData::Binary {
                op: BinaryOp::Add,
                lhs: a,
                rhs: b,
            },
            Type::Int,
            span,
        );
        f.set_terminator(entry, Terminator::Ret(sum));

        assert_eq!(f.value_count(), 3);
        assert_eq!(a.index(), 0);
        assert_eq!(sum.index(), 2);
        assert_eq!(f.value_type(sum), Type::Int);
        assert_eq!(f.block(entry).body, vec![a, b, sum]);
        assert_eq!(f.block(entry).terminator, Some(Terminator::Ret(sum)));
    }

    #[test]
    fn module_holds_functions() {
        let mut m = Module::new();
        m.add_function(Function::new("a", Type::Int));
        m.add_function(Function::new("b", Type::Int));
        assert_eq!(m.functions().len(), 2);
        assert_eq!(m.functions()[1].name, "b");
    }
}
