// Exports a searchable index of the analysed program into re/exports/ (gitignored):
// functions.tsv, strings.tsv (with referencing functions), vftables.tsv (MSVC RTTI classes).
// Usage: analyzeHeadless <proj> sekiro -process -noanalysis -scriptPath tools/ghidra
//        -postScript ExportIndex.java <outDir>
// @category sekiro-rs

import java.io.File;
import java.io.PrintWriter;
import java.util.LinkedHashSet;
import java.util.Set;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Data;
import ghidra.program.model.listing.DataIterator;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionIterator;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.Symbol;
import ghidra.program.model.symbol.SymbolIterator;

public class ExportIndex extends GhidraScript {

    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        File out = new File(args.length > 0 ? args[0] : "re/exports");
        out.mkdirs();
        long base = currentProgram.getImageBase().getOffset();

        try (PrintWriter w = new PrintWriter(new File(out, "functions.tsv"), "UTF-8")) {
            w.println("rva\tsize\tname\tnamespace");
            FunctionIterator it = currentProgram.getFunctionManager().getFunctions(true);
            while (it.hasNext() && !monitor.isCancelled()) {
                Function f = it.next();
                w.printf("%x\t%d\t%s\t%s%n", f.getEntryPoint().getOffset() - base,
                    f.getBody().getNumAddresses(), f.getName(), f.getParentNamespace().getName(true));
            }
        }

        try (PrintWriter w = new PrintWriter(new File(out, "strings.tsv"), "UTF-8")) {
            w.println("rva\tstring\tref_functions");
            DataIterator it = currentProgram.getListing().getDefinedData(true);
            while (it.hasNext() && !monitor.isCancelled()) {
                Data d = it.next();
                if (!d.hasStringValue()) {
                    continue;
                }
                Object v = d.getValue();
                if (v == null) {
                    continue;
                }
                String s = v.toString().replace("\t", "\\t").replace("\n", "\\n").replace("\r", "\\r");
                Set<String> refs = new LinkedHashSet<>();
                for (Reference r : getReferencesTo(d.getAddress())) {
                    Function f = getFunctionContaining(r.getFromAddress());
                    refs.add(f == null ? "?" + Long.toHexString(r.getFromAddress().getOffset() - base)
                        : Long.toHexString(f.getEntryPoint().getOffset() - base));
                }
                w.printf("%x\t%s\t%s%n", d.getAddress().getOffset() - base, s, String.join(",", refs));
            }
        }

        try (PrintWriter w = new PrintWriter(new File(out, "vftables.tsv"), "UTF-8")) {
            w.println("rva\tclass");
            SymbolIterator it = currentProgram.getSymbolTable().getSymbolIterator("vftable", true);
            while (it.hasNext() && !monitor.isCancelled()) {
                Symbol s = it.next();
                Address a = s.getAddress();
                w.printf("%x\t%s%n", a.getOffset() - base, s.getParentNamespace().getName(true));
            }
        }
        println("exported index to " + out.getAbsolutePath());
    }
}
