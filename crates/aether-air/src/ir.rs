//! AIR data structures: modules, functions, blocks, instructions, and values.

use aether_source::Span;

/// An AIR type. Two primitive types exist today; more arrive with the type
/// system.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Type {
    /// The 64-bit signed integer type, `int`.
    Int,
    /// The boolean type, `bool`.
    Bool,
}

impl Type {
    /// The type's textual name (e.g. `"int"`).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Type::Int => "int",
            Type::Bool => "bool",
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
    /// Integer negation (`int` → `int`).
    Neg,
    /// Logical negation (`bool` → `bool`).
    Not,
}

impl UnaryOp {
    /// The mnemonic used in AIR's textual form (e.g. `"neg"`).
    #[must_use]
    pub fn mnemonic(self) -> &'static str {
        match self {
            UnaryOp::Neg => "neg",
            UnaryOp::Not => "not",
        }
    }
}

/// An integer comparison operator. Each compares two operands and produces a
/// [`Type::Bool`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CmpOp {
    /// Equal, `==`.
    Eq,
    /// Not equal, `!=`.
    Ne,
    /// Less than, `<`.
    Lt,
    /// Less than or equal, `<=`.
    Le,
    /// Greater than, `>`.
    Gt,
    /// Greater than or equal, `>=`.
    Ge,
}

impl CmpOp {
    /// The mnemonic used in AIR's textual form (e.g. `"lt"`, as in `icmp lt`).
    #[must_use]
    pub fn mnemonic(self) -> &'static str {
        match self {
            CmpOp::Eq => "eq",
            CmpOp::Ne => "ne",
            CmpOp::Lt => "lt",
            CmpOp::Le => "le",
            CmpOp::Gt => "gt",
            CmpOp::Ge => "ge",
        }
    }

    /// Whether this comparison is an equality test (`==`/`!=`), which accepts
    /// operands of any single type, as opposed to a relational test (`<`, `<=`,
    /// `>`, `>=`), which requires integer operands.
    #[must_use]
    pub fn is_equality(self) -> bool {
        matches!(self, CmpOp::Eq | CmpOp::Ne)
    }
}

/// A reference to an SSA value.
///
/// A `Value` is an index into its [`Function`]'s value table. Every value is
/// defined either by an instruction (its result) or as a block parameter — see
/// [`ValueDef`].
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
///
/// Not `Copy`: [`InstData::Call`] carries an owned callee name and argument list.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum InstData {
    /// An integer constant.
    IConst(i64),
    /// A boolean constant.
    BConst(bool),
    /// A unary operation on one value.
    Unary {
        /// The operator.
        op: UnaryOp,
        /// The operand.
        operand: Value,
    },
    /// A binary arithmetic operation on two values (producing an integer).
    Binary {
        /// The operator.
        op: BinaryOp,
        /// The left operand.
        lhs: Value,
        /// The right operand.
        rhs: Value,
    },
    /// An integer comparison of two values (producing a boolean).
    ICmp {
        /// The comparison operator.
        op: CmpOp,
        /// The left operand.
        lhs: Value,
        /// The right operand.
        rhs: Value,
    },
    /// A call to another function by name, producing its return value.
    ///
    /// The callee is referenced by name (provisional, ADR-0021): resolution lives
    /// in lowering until the dedicated name-resolution pass (M9). The arguments
    /// bind the callee's entry-block parameters.
    Call {
        /// The called function's name.
        callee: String,
        /// The argument values, in order.
        args: Vec<Value>,
    },
}

/// How a [`Value`] is defined.
///
/// Every value is either the result of an instruction or a parameter of a block
/// (an SSA merge point — see ADR-0017). Because each instruction defines exactly
/// one value, the instruction's data is stored inline here rather than in a
/// separate arena.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ValueDef {
    /// The result of an instruction with this data.
    Inst(InstData),
    /// The `index`-th parameter of `block`, bound from the arguments a
    /// predecessor supplies on the edge it takes into `block`.
    Param {
        /// The block this parameter belongs to.
        block: Block,
        /// The parameter's position in the block's parameter list.
        index: usize,
    },
}

/// A value's definition, its type, and the source span it came from.
#[derive(Clone, Debug)]
struct ValueData {
    def: ValueDef,
    ty: Type,
    span: Span,
}

/// A branch edge: the target block and the arguments passed to its parameters.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BranchTarget {
    /// The destination block.
    pub block: Block,
    /// The values supplied to the destination's parameters, in order.
    pub args: Vec<Value>,
}

impl BranchTarget {
    /// A branch to `block` passing no arguments.
    #[must_use]
    pub fn new(block: Block) -> BranchTarget {
        BranchTarget {
            block,
            args: Vec::new(),
        }
    }

    /// A branch to `block` passing `args`.
    #[must_use]
    pub fn with_args(block: Block, args: Vec<Value>) -> BranchTarget {
        BranchTarget { block, args }
    }
}

/// How a basic block ends and transfers control.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Terminator {
    /// Return a value from the function.
    Ret(Value),
    /// Unconditionally branch to another block.
    Br(BranchTarget),
    /// Branch to `then_branch` if `cond` (a `bool`) is true, else `else_branch`.
    CondBr {
        /// The boolean condition selecting the successor.
        cond: Value,
        /// The edge taken when `cond` is true.
        then_branch: BranchTarget,
        /// The edge taken when `cond` is false.
        else_branch: BranchTarget,
    },
}

impl Terminator {
    /// The blocks this terminator may transfer control to, in order.
    #[must_use]
    pub fn successors(&self) -> Vec<Block> {
        match self {
            Terminator::Ret(_) => Vec::new(),
            Terminator::Br(target) => vec![target.block],
            Terminator::CondBr {
                then_branch,
                else_branch,
                ..
            } => vec![then_branch.block, else_branch.block],
        }
    }
}

/// The contents of a basic block: its parameters, the values it computes, and a
/// terminator.
#[derive(Clone, Debug, Default)]
pub struct BlockData {
    /// The block's parameters (SSA merge points), in order. Empty for the entry
    /// block and for blocks with a single predecessor that need no merge.
    pub params: Vec<Value>,
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
    /// Value table, addressed by [`Value`]: instruction results and block
    /// parameters alike.
    values: Vec<ValueData>,
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
            values: Vec::new(),
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

    /// Append a **function** parameter of type `ty`, returning the [`Value`] it
    /// defines. A function's parameters are the parameters of its entry block,
    /// bound from a call's arguments just as block parameters are bound from a
    /// branch's arguments (ADR-0021). This is a convenience over
    /// [`append_block_param`](Function::append_block_param) on the entry block.
    pub fn append_param(&mut self, ty: Type, span: Span) -> Value {
        self.append_block_param(self.entry, ty, span)
    }

    /// The function's parameters (the entry block's parameters), in order.
    #[must_use]
    pub fn params(&self) -> &[Value] {
        &self.blocks[self.entry.index()].params
    }

    /// Append a parameter of type `ty` to `block`, returning the [`Value`] it
    /// defines. Predecessors supply a matching argument on each edge into `block`.
    pub fn append_block_param(&mut self, block: Block, ty: Type, span: Span) -> Value {
        let index = self.blocks[block.index()].params.len();
        let value = self.new_value(ValueData {
            def: ValueDef::Param { block, index },
            ty,
            span,
        });
        self.blocks[block.index()].params.push(value);
        value
    }

    /// Append an instruction to `block`, returning the [`Value`] it defines.
    pub fn push_inst(&mut self, block: Block, data: InstData, ty: Type, span: Span) -> Value {
        let value = self.new_value(ValueData {
            def: ValueDef::Inst(data),
            ty,
            span,
        });
        self.blocks[block.index()].body.push(value);
        value
    }

    fn new_value(&mut self, data: ValueData) -> Value {
        let value = Value::from_index(self.values.len());
        self.values.push(data);
        value
    }

    /// Set the terminator of `block`.
    pub fn set_terminator(&mut self, block: Block, terminator: Terminator) {
        self.blocks[block.index()].terminator = Some(terminator);
    }

    /// How `value` is defined.
    #[must_use]
    pub fn value_def(&self, value: Value) -> &ValueDef {
        &self.values[value.index()].def
    }

    /// The type of `value`.
    #[must_use]
    pub fn value_type(&self, value: Value) -> Type {
        self.values[value.index()].ty
    }

    /// The source span `value` was lowered from.
    #[must_use]
    pub fn value_span(&self, value: Value) -> Span {
        self.values[value.index()].span
    }

    /// The number of values (instruction results and block parameters) defined in
    /// this function.
    #[must_use]
    pub fn value_count(&self) -> usize {
        self.values.len()
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

    /// The function with the given name, if any. Used to resolve calls until a
    /// dedicated name-resolution pass turns callee names into direct references
    /// (M9); a linear scan is adequate at current scale.
    #[must_use]
    pub fn function_by_name(&self, name: &str) -> Option<&Function> {
        self.functions.iter().find(|f| f.name == name)
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
    fn block_parameters_are_values() {
        let span = dummy_span();
        let mut f = Function::new("f", Type::Int);
        let entry = f.entry();
        let join = f.append_block();
        let p = f.append_block_param(join, Type::Int, span);
        let c = f.push_inst(entry, InstData::IConst(1), Type::Int, span);
        f.set_terminator(
            entry,
            Terminator::Br(BranchTarget::with_args(join, vec![c])),
        );
        f.set_terminator(join, Terminator::Ret(p));

        // The parameter is a value defined as the 0th param of `join`.
        assert_eq!(f.value_type(p), Type::Int);
        assert_eq!(
            f.value_def(p),
            &ValueDef::Param {
                block: join,
                index: 0
            }
        );
        assert_eq!(f.block(join).params, vec![p]);
        // The predecessor passes `c` as the join's argument.
        assert_eq!(
            f.block(entry).terminator,
            Some(Terminator::Br(BranchTarget::with_args(join, vec![c])))
        );
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
