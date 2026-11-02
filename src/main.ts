import { createApp } from "vue";
import App from "./App.vue";
import "./styles.css";

if (window.location.hash.includes("overlay")) {
  document.documentElement.classList.add("overlay-mode");
  document.body.classList.add("overlay-body");
}

createApp(App).mount("#app");

