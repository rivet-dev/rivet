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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faNfcSlash.js
var require_faNfcSlash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faNfcSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "nfc-slash";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e1fc";
    var svgPathData = "M38.8 5.1C28.4-3.1 13.3-1.2 5.1 9.2S-1.2 34.7 9.2 42.9l592 464c10.4 8.2 25.5 6.3 33.7-4.1s6.3-25.5-4.1-33.7l-86.8-68L544 96c0-35.3-28.7-64-64-64L160 32c-21.6 0-40.7 10.7-52.3 27.1L38.8 5.1zM173.5 110.7c8.7-9 21-14.7 34.5-14.7l32 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-32 0c-3.3 0-6.4 1-8.9 2.7l-25.6-20.1zM302.1 211.5c.6-.3 1.3-.5 1.9-.7l0-66.7c0-26.5 21.5-48 48-48l80 0c26.5 0 48 21.5 48 48l0 206.9-32-25.1L448 144c0-8.8-7.2-16-16-16l-80 0c-8.8 0-16 7.2-16 16l0 66.7c18.6 6.6 32 24.4 32 45.3c0 2.3-.2 4.6-.5 6.8l-65.5-51.3zM486.9 479.6L406.2 416 208 416c-26.5 0-48-21.5-48-48l0-145.9L96 171.6 96 416c0 35.3 28.7 64 64 64l320 0c2.3 0 4.6-.1 6.9-.4zM208 384l157.5 0L192 247.3 192 368c0 8.8 7.2 16 16 16z";
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
    exports.faNfcSlash = exports.definition;
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

// src/_entries/faNfcSlash.js
var import_faNfcSlash = __toESM(require_faNfcSlash());
var export_faNfcSlash = import_faNfcSlash.definition;
export {
  export_faNfcSlash as faNfcSlash
};
