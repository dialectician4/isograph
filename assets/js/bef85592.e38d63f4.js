"use strict";(self.webpackChunkdocs=self.webpackChunkdocs||[]).push([[211],{4718:(e,n,i)=>{i.r(n),i.d(n,{assets:()=>a,contentTitle:()=>c,default:()=>h,frontMatter:()=>s,metadata:()=>r,toc:()=>l});var o=i(5250),t=i(1340);const s={sidebar_position:4},c="The Isograph config",r={id:"isograph-config",title:"The Isograph config",description:"If anything in this document is inaccurate, please consult the source of truth:",source:"@site/docs/isograph-config.md",sourceDirName:".",slug:"/isograph-config",permalink:"/docs/isograph-config",draft:!1,unlisted:!1,tags:[],version:"current",sidebarPosition:4,frontMatter:{sidebar_position:4},sidebar:"tutorialSidebar",previous:{title:"Magic mutation fields",permalink:"/docs/magic-mutation-fields"},next:{title:"FAQ",permalink:"/docs/faq"}},a={},l=[{value:"Config file location and name",id:"config-file-location-and-name",level:2},{value:"Config file contents",id:"config-file-contents",level:2}];function d(e){const n={a:"a",admonition:"admonition",code:"code",h1:"h1",h2:"h2",li:"li",p:"p",pre:"pre",ul:"ul",...(0,t.a)(),...e.components};return(0,o.jsxs)(o.Fragment,{children:[(0,o.jsx)(n.h1,{id:"the-isograph-config",children:"The Isograph config"}),"\n",(0,o.jsx)(n.admonition,{type:"info",children:(0,o.jsxs)(n.p,{children:["If anything in this document is inaccurate, please consult the source of truth:\n",(0,o.jsx)(n.a,{href:"https://github.com/isographlabs/isograph/blob/main/crates/isograph_cli/src/config.rs",children:"the code in GitHub"}),". Make sure to change ",(0,o.jsx)(n.code,{children:"main"})," in the URL to the specific commit that you actually installed."]})}),"\n",(0,o.jsx)(n.h2,{id:"config-file-location-and-name",children:"Config file location and name"}),"\n",(0,o.jsxs)(n.p,{children:["The file should be named ",(0,o.jsx)(n.code,{children:"isograph.config.json"})," and located at the root of your project."]}),"\n",(0,o.jsx)(n.admonition,{type:"warning",children:(0,o.jsxs)(n.p,{children:[(0,o.jsx)(n.code,{children:"yarn iso --config $PATH"})," will work if the config is not named ",(0,o.jsx)(n.code,{children:"isograph.config.json"}),", or is not found in the root of the project. But the babel plugin will not (yet!)"]})}),"\n",(0,o.jsx)(n.h2,{id:"config-file-contents",children:"Config file contents"}),"\n",(0,o.jsx)(n.p,{children:"An example (complete) Isograph config is as follows:"}),"\n",(0,o.jsx)(n.pre,{children:(0,o.jsx)(n.code,{className:"language-json",children:'{\n  "project_root": "./src/components",\n  "artifact_directory": "./src",\n  "schema": "./backend/schema.graphql",\n  "schema_extensions": ["./backend/schema-extension.graphql"],\n  "options": {\n    "on_invalid_id_type": "error"\n  }\n}\n'})}),"\n",(0,o.jsxs)(n.ul,{children:["\n",(0,o.jsx)(n.li,{children:"All paths are relative."}),"\n",(0,o.jsxs)(n.li,{children:[(0,o.jsx)(n.code,{children:"schema"})," and ",(0,o.jsx)(n.code,{children:"schema_extensions"})," take relative paths to files, not to folders."]}),"\n",(0,o.jsxs)(n.li,{children:["Only ",(0,o.jsx)(n.code,{children:"project_root"}),", ",(0,o.jsx)(n.code,{children:"artifact_directory"})," and ",(0,o.jsx)(n.code,{children:"schema"})," are required."]}),"\n",(0,o.jsxs)(n.li,{children:["Valid values for ",(0,o.jsx)(n.code,{children:"on_invalid_id_type"})," are ",(0,o.jsx)(n.code,{children:"ignore"}),", ",(0,o.jsx)(n.code,{children:"warning"})," and ",(0,o.jsx)(n.code,{children:"error"}),"."]}),"\n"]})]})}function h(e={}){const{wrapper:n}={...(0,t.a)(),...e.components};return n?(0,o.jsx)(n,{...e,children:(0,o.jsx)(d,{...e})}):d(e)}},1340:(e,n,i)=>{i.d(n,{Z:()=>r,a:()=>c});var o=i(79);const t={},s=o.createContext(t);function c(e){const n=o.useContext(s);return o.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function r(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(t):e.components||t:c(e.components),o.createElement(s.Provider,{value:n},e.children)}}}]);