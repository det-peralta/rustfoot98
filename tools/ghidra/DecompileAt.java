// args: <outfile> <addr> [<addr>...]  -> decompiled C for the function containing each address
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.*;
import ghidra.program.model.address.*;
import ghidra.program.model.listing.*;
import java.io.*;
public class DecompileAt extends GhidraScript {
  @Override public void run() throws Exception {
    String[] a = getScriptArgs();
    DecompInterface di = new DecompInterface(); di.openProgram(currentProgram);
    try (PrintWriter w = new PrintWriter(new FileWriter(a[0], true))) {
      for (int i = 1; i < a.length; i++) {
        Address ad = currentProgram.getAddressFactory().getAddress(a[i]);
        Function f = getFunctionContaining(ad);
        if (f == null) { f = createFunction(ad, null); }
        w.println("// ===== " + a[i] + " " + (f == null ? "NO FUNCTION" : f.getName() + " @ " + f.getEntryPoint()));
        if (f == null) continue;
        DecompileResults r = di.decompileFunction(f, 60, monitor);
        w.println(r.decompileCompleted() ? r.getDecompiledFunction().getC() : "// decompile failed: " + r.getErrorMessage());
      }
    }
  }
}
