// Lists callers (and callees) of functions by RVA into <outDir>/<rva>.xrefs.txt.
// Output stays in re/ (gitignored).
// Usage: headless.ps1 Callers.java re/exports/xrefs <rva> [<rva> ...]
// @category sekiro-rs

import java.io.File;
import java.io.PrintWriter;
import java.util.Set;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.Reference;

public class Callers extends GhidraScript {

    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        File out = new File(args[0]);
        out.mkdirs();
        long base = currentProgram.getImageBase().getOffset();
        for (int i = 1; i < args.length; i++) {
            long rva = Long.parseLong(args[i].replace("0x", ""), 16);
            Address a = toAddr(base + rva);
            Function f = getFunctionContaining(a);
            try (PrintWriter w = new PrintWriter(new File(out, String.format("%x.xrefs.txt", rva)), "UTF-8")) {
                for (Reference r : getReferencesTo(a)) {
                    Address from = r.getFromAddress();
                    Function c = getFunctionContaining(from);
                    long crva = c == null ? -1 : c.getEntryPoint().getOffset() - base;
                    w.printf("to\t%x\t%s\t%x%n", from.getOffset() - base, r.getReferenceType(), crva);
                }
                if (f != null) {
                    Set<Function> callees = f.getCalledFunctions(monitor);
                    for (Function c : callees) {
                        w.printf("callee\t%x\t%s%n", c.getEntryPoint().getOffset() - base, c.getName());
                    }
                }
            }
        }
    }
}
