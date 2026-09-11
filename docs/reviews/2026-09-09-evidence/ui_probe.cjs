const fs=require('fs');const vm=require('vm');
const ts=require('C:/code/pocx/satchel/satchel/ui/node_modules/typescript');
let source=fs.readFileSync('C:/code/pocx/satchel/satchel/ui/src/format.ts','utf8').replace('import { tr } from "./i18n";','const tr = (s: string) => s;');
const compiled=ts.transpileModule(source,{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText;
for(const locale of ['de-DE','en-US']) {
 const ex={};const intl={...Intl,NumberFormat:class extends Intl.NumberFormat {constructor(_l,opts){super(locale,opts)}}};
 vm.runInNewContext(compiled,{exports:ex,Intl:intl});
 const input=locale==='de-DE'?'0.001':'0,001';const result=ex.sanitizeAmountInput(input);
 console.log(JSON.stringify({locale,input,sanitized:result,wire:ex.canonicalAmount(result),parsed:ex.parseAmount(result)}));
 for(const s of [{state:'completed'},{state:'completed',progress:{watching:'settlement'}},{state:'refunded',progress:{watching:'settlement'}}])console.log(JSON.stringify({swap:s,active:ex.isActive(s)}));
}
