// args: <outfile> <needle> : lists instructions whose text contains the needle, with their function
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.*;
import java.io.*;
public class FindOperand extends GhidraScript {
  @Override public void run() throws Exception {
    String[] a = getScriptArgs();
    try (PrintWriter w = new PrintWriter(a[0])) {
      for (Instruction i = currentProgram.getListing().getInstructions(true).next(); i != null; i = i.getNext()) {
        String s = i.toString();
        if (s.contains(a[1])) {
          Function f = getFunctionContaining(i.getAddress());
          w.printf("%s\t%s\t%s%n", i.getAddress(), f == null ? "-" : f.getEntryPoint(), s);
        }
      }
    }
  }
}
