// Lists every call to Random (10c0:212d) with the immediate pushed just before it.
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.*;
import ghidra.program.model.listing.*;
import ghidra.program.model.symbol.*;
import ghidra.program.model.scalar.*;
import java.io.*;
public class RandomCalls extends GhidraScript {
  @Override public void run() throws Exception {
    Address rnd = currentProgram.getAddressFactory().getAddress(getScriptArgs()[1]);
    try (PrintWriter w = new PrintWriter(getScriptArgs()[0])) {
      for (Reference r : getReferencesTo(rnd)) {
        Address from = r.getFromAddress();
        Function f = getFunctionContaining(from);
        String imm = "?";
        Instruction i = getInstructionBefore(from);
        for (int k = 0; k < 4 && i != null; k++, i = i.getPrevious()) {
          String m = i.getMnemonicString();
          if ((m.equals("PUSH") || m.equals("MOV")) && i.getNumOperands() >= 1) {
            Object[] ops = i.getOpObjects(i.getNumOperands() - 1);
            if (ops.length == 1 && ops[0] instanceof Scalar) { imm = "" + ((Scalar) ops[0]).getUnsignedValue(); break; }
          }
        }
        w.printf("%s\t%s\t%s%n", from, f == null ? "-" : f.getEntryPoint(), imm);
      }
    }
  }
}
