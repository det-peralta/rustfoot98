// args: <addrlist> <outC> : disassembles and creates a function at every listed address (the ones Ghidra's analysis missed),
// then decompiles the new functions into <outC>.
import ghidra.app.script.GhidraScript;
import ghidra.app.cmd.disassemble.DisassembleCommand;
import ghidra.app.cmd.function.CreateFunctionCmd;
import ghidra.app.decompiler.*;
import ghidra.program.model.address.*;
import ghidra.program.model.listing.*;
import java.io.*;
import java.nio.file.*;
import java.util.*;
public class FixFunctions extends GhidraScript {
  @Override public void run() throws Exception {
    String[] a = getScriptArgs();
    List<String> lines = Files.readAllLines(Paths.get(a[0]));
    AddressFactory af = currentProgram.getAddressFactory();
    List<Function> made = new ArrayList<>();
    for (String l : lines) {
      l = l.trim(); if (l.isEmpty()) continue;
      Address ad = af.getAddress(l);
      if (getFunctionAt(ad) != null) continue;
      if (getInstructionAt(ad) == null) new DisassembleCommand(ad, null, true).applyTo(currentProgram);
      Function f = getFunctionContaining(ad);
      if (f == null || !f.getEntryPoint().equals(ad)) {
        new CreateFunctionCmd(ad).applyTo(currentProgram, monitor);
        f = getFunctionAt(ad);
      }
      if (f != null) made.add(f);
    }
    println("functions created: " + made.size());
    DecompInterface di = new DecompInterface(); di.openProgram(currentProgram);
    try (PrintWriter w = new PrintWriter(new FileWriter(a[1]))) {
      for (Function f : made) {
        w.println("// ===== " + f.getEntryPoint() + " " + f.getName());
        DecompileResults r = di.decompileFunction(f, 30, monitor);
        w.println(r.decompileCompleted() ? r.getDecompiledFunction().getC() : "// failed");
      }
    }
  }
}
