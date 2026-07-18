var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPenClipSlash.js
var require_faPenClipSlash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPenClipSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pen-clip-slash";
    var width = 640;
    var height = 512;
    var aliases = ["pen-alt-slash"];
    var unicode = "e20f";
    var svgPathData = "M38.8 5.1C28.4-3.1 13.3-1.2 5.1 9.2S-1.2 34.7 9.2 42.9l592 464c10.4 8.2 25.5 6.3 33.7-4.1s6.3-25.5-4.1-33.7L409.9 296l55-55 17-17-1-1-16-16-96-96-17-17L338.9 81c-28.1-28.1-73.7-28.1-101.8 0l-44.6 44.6L38.8 5.1zM230.5 155.4L271 114.9c9.4-9.4 24.6-9.4 33.9 0L318.1 128l-53.8 53.8-33.7-26.4zM341.2 364.8L195.8 250.3l-32.3 32.3c-48 48-80.8 109.2-94.1 175.8l-5 25c-1.6 7.9 .9 16 6.6 21.7s13.8 8.1 21.7 6.6l25-5c66.6-13.3 127.8-46.1 175.8-94.1l47.7-47.7zM517.3 19.3c-25-25-65.5-25-90.5 0L374.6 71.4l17 17 96 96 16 16s0 0 0 0l1 1s0 0 0 0l52.1-52.1c25-25 25-65.5 0-90.5L517.3 19.3z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faPenClipSlash = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faPenClipSlash.js
var import_faPenClipSlash = __toESM(require_faPenClipSlash());
var export_faPenClipSlash = import_faPenClipSlash.definition;
export {
  export_faPenClipSlash as faPenClipSlash
};
