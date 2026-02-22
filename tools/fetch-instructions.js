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

console.log("#[derive(Copy, Clone, Debug)]");
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

console.log("#[derive(Copy, Clone, Debug)]");
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
console.log("  pub name: OperandName,");
console.log("  pub register: Option<Register>,");
console.log("  pub immediate: bool,");
console.log("  pub bytes: u8,");
console.log("  pub increment: bool,");
console.log("  pub decrement: bool,");
console.log("}");
console.log();

console.log("pub struct Instruction {");
console.log("  pub mnemonic: Mnemonic,");
console.log("  pub cycles: [u8; 2],");
console.log("  pub bytes: u8,");
console.log("  pub operands: [Operand; 3],");
console.log("  pub total_operands: u8,");
console.log("  pub total_cycles: u8,");
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
