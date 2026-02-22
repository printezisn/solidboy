const fillArray = (arr, defaultValue, length) => {
  const newArr = [...arr];

  while (newArr.length < length) {
    newArr.push(defaultValue);
  }

  return newArr;
};

const getOperandName = (str) => {
  if (!isNaN(Number(str))) return `NUM${str}`.toUpperCase();
  return str.replace(/\$/g, "DOLLAR").replace(/_/g, "").toUpperCase();
};

const logInstructions = (instructions) => {
  for (const key in instructions) {
    const instruction = instructions[key];

    const allOperands = fillArray(instruction.operands, { name: "NONE" }, 3);
    const allCycles = fillArray(instruction.cycles, 0, 2);

    const operands = allOperands
      .map((operand) => {
        const register = REGISTERS.includes(operand.name)
          ? `Some(Register::${operand.name})`
          : "None";

        return (
          "      Operand {\n" +
          `        name: OperandName::${getOperandName(operand.name)},\n` +
          `        register: ${register},\n` +
          `        immediate: ${Boolean(operand.immediate)},\n` +
          `        bytes: ${operand.bytes || 0},\n` +
          `        increment: ${Boolean(operand.increment)},\n` +
          `        decrement: ${Boolean(operand.decrement)},\n` +
          "      },\n"
        );
      })
      .join("");

    console.log(
      "  Instruction {\n" +
        `    mnemonic: Mnemonic::${instruction.mnemonic.toUpperCase().replace(/_/g, "")},\n` +
        `    cycles: [${allCycles.join(", ")}],\n` +
        `    bytes: ${instruction.bytes},\n` +
        `    operands: [\n${operands}    ],\n` +
        `    total_operands: ${instruction.operands.length},\n` +
        `    total_cycles: ${instruction.cycles.length},\n` +
        "  },",
    );
  }
};

const response = await fetch("https://gbdev.io/gb-opcodes/Opcodes.json");
const result = await response.json();

const REGISTERS = [
  "A",
  "B",
  "C",
  "D",
  "E",
  "F",
  "H",
  "L",
  "AF",
  "BC",
  "DE",
  "HL",
  "SP",
  "PC",
];

console.log("use super::registers::Register;");
console.log();

console.log("pub enum Mnemonic {");
Array.from(
  new Set([
    ...Object.values(result.unprefixed).map(
      (instruction) => instruction.mnemonic,
    ),
    ...Object.values(result.cbprefixed).map(
      (instruction) => instruction.mnemonic,
    ),
  ]),
).forEach((mnemonic) => {
  console.log(`  ${mnemonic.toUpperCase().replace(/_/g, "")},`);
});
console.log("}");
console.log();

console.log("pub enum OperandName {");
Array.from(
  new Set([
    ...Object.values(result.unprefixed)
      .map((instruction) => instruction.operands.map((operand) => operand.name))
      .flat(),
    ...Object.values(result.cbprefixed)
      .map((instruction) => instruction.operands.map((operand) => operand.name))
      .flat(),
  ]),
).forEach((operandName) => {
  console.log(`  ${getOperandName(operandName)},`);
});
console.log("  NONE,");
console.log("}");
console.log();

console.log("pub struct Operand {");
console.log("  name: OperandName,");
console.log("  register: Option<Register>,");
console.log("  immediate: bool,");
console.log("  bytes: u8,");
console.log("  increment: bool,");
console.log("  decrement: bool,");
console.log("}");
console.log();

console.log("pub struct Instruction {");
console.log("  mnemonic: Mnemonic,");
console.log("  cycles: [u8; 2],");
console.log("  bytes: u8,");
console.log("  operands: [Operand; 3],");
console.log("  total_operands: u8,");
console.log("  total_cycles: u8,");
console.log("}");
console.log();

console.log("pub const PREFIXED_INSTRUCTIONS: [Instruction; 256] = [");
logInstructions(result.unprefixed);
console.log("];");
console.log();

console.log("pub const CBPREFIXED_INSTRUCTIONS: [Instruction; 256] = [");
logInstructions(result.cbprefixed);
console.log("];");
console.log();
