// Decompiles functions by RVA into <outDir>/<rva>.c for study. Output stays in re/ (gitignored);
// never paste it into tracked source.
// Usage: analyzeHeadless <proj> sekiro -process -noanalysis -readOnly -scriptPath tools/ghidra
//        -postScript Decompile.java <outDir> <rva> [<rva> ...]
// @category sekiro-rs

import java.io.File;
import java.io.PrintWriter;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;

public class Decompile extends GhidraScript {

    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        File out = new File(args[0]);
        out.mkdirs();
        long base = currentProgram.getImageBase().getOffset();
        DecompInterface d = new DecompInterface();
        d.openProgram(currentProgram);
        for (int i = 1; i < args.length; i++) {
            long rva = Long.parseLong(args[i].replace("0x", ""), 16);
            Address a = toAddr(base + rva);
            Function f = getFunctionContaining(a);
            if (f == null) {
                f = createFunction(a, null);
            }
            if (f == null) {
                println("no function at " + args[i]);
                continue;
            }
            DecompileResults r = d.decompileFunction(f, 120, monitor);
            String text = r.decompileCompleted() ? r.getDecompiledFunction().getC() : "// failed: " + r.getErrorMessage();
            try (PrintWriter w = new PrintWriter(new File(out, String.format("%x.c", rva)), "UTF-8")) {
                w.printf("// %s at rva %x%n", f.getName(), f.getEntryPoint().getOffset() - base);
                w.print(text);
            }
            println("decompiled " + f.getName());
        }
        d.dispose();
    }
}
