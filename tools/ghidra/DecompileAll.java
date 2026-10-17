// Decompiles every function to one file (args: outfile). Slow; run once.
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.*;
import ghidra.program.model.listing.*;
import java.io.*;
public class DecompileAll extends GhidraScript {
  @Override public void run() throws Exception {
    DecompInterface di = new DecompInterface(); di.openProgram(currentProgram);
    int n = 0;
    try (PrintWriter w = new PrintWriter(new FileWriter(getScriptArgs()[0]))) {
      for (Function f : currentProgram.getFunctionManager().getFunctions(true)) {
        if (monitor.isCancelled()) break;
        w.println("// ===== " + f.getEntryPoint() + " " + f.getName());
        DecompileResults r = di.decompileFunction(f, 20, monitor);
        w.println(r.decompileCompleted() ? r.getDecompiledFunction().getC() : "// failed");
        if (++n % 200 == 0) println("done " + n);
      }
    }
  }
}
