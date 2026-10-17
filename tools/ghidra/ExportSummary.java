// Exports memory map, functions and defined strings to analysis/out/
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.*;
import ghidra.program.model.mem.*;
import ghidra.program.model.symbol.*;
import java.io.*;

public class ExportSummary extends GhidraScript {
  @Override public void run() throws Exception {
    File dir = new File(getScriptArgs().length > 0 ? getScriptArgs()[0] : "analysis/out");
    dir.mkdirs();
    try (PrintWriter w = new PrintWriter(new File(dir, "memory.txt"))) {
      for (MemoryBlock b : currentProgram.getMemory().getBlocks())
        w.printf("%s\t%s\t%d\t%s%s%s%n", b.getName(), b.getStart(), b.getSize(),
          b.isRead()?"r":"-", b.isWrite()?"w":"-", b.isExecute()?"x":"-");
    }
    int n = 0;
    try (PrintWriter w = new PrintWriter(new File(dir, "functions.txt"))) {
      for (Function f : currentProgram.getFunctionManager().getFunctions(true)) {
        w.printf("%s\t%s\t%d%n", f.getEntryPoint(), f.getName(), f.getBody().getNumAddresses()); n++;
      }
    }
    int s = 0;
    try (PrintWriter w = new PrintWriter(new File(dir, "strings.txt"))) {
      for (Data d : currentProgram.getListing().getDefinedData(true))
        if (d.hasStringValue()) { w.printf("%s\t%s%n", d.getAddress(), d.getDefaultValueRepresentation()); s++; }
    }
    try (PrintWriter w = new PrintWriter(new File(dir, "symbols.txt"))) {
      for (Symbol y : currentProgram.getSymbolTable().getAllSymbols(false))
        if (y.getSource() != SourceType.DEFAULT) w.printf("%s\t%s\t%s%n", y.getAddress(), y.getSymbolType(), y.getName(true));
    }
    println("functions=" + n + " strings=" + s);
  }
}
