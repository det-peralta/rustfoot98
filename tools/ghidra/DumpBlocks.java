// args: <outdir> -> writes every initialized memory block as <seg>_<name>.bin (for offline analysis)
import ghidra.app.script.GhidraScript;
import ghidra.program.model.mem.*;
import java.io.*;
public class DumpBlocks extends GhidraScript {
  @Override public void run() throws Exception {
    File dir = new File(getScriptArgs()[0]); dir.mkdirs();
    for (MemoryBlock b : currentProgram.getMemory().getBlocks()) {
      if (!b.isInitialized()) continue;
      byte[] d = new byte[(int) b.getSize()];
      b.getBytes(b.getStart(), d);
      String s = b.getStart().toString().replace(':', '_');
      try (FileOutputStream f = new FileOutputStream(new File(dir, s + "_" + b.getName().replaceAll("[^A-Za-z0-9]", "") + ".bin"))) { f.write(d); }
    }
  }
}
