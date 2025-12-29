const r=(t,u)=>{let e;return{update:o=>{clearTimeout(e),e=setTimeout(()=>t?.(o),u)},destroy:()=>clearTimeout(e)}};export{r as u};
