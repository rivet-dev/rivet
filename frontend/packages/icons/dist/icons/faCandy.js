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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCandy.js
var require_faCandy = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCandy.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "candy";
    var width = 640;
    var height = 512;
    var aliases = [127852];
    var unicode = "e3e7";
    var svgPathData = "M318.1 64c-24.5 .2-47.9 5.1-69.4 13.7L419.9 420c20.3-12.4 38.2-28.5 52.6-47.3L318.1 64zM492.3 340.8c4.7-9.6 8.7-19.6 11.8-30l93.2 46.6c10.5 5.3 23.3 2.1 30.2-7.4s5.8-22.7-2.5-31l-29.6-29.6 29.1-10.9C633.8 275 640 266 640 256s-6.2-19-15.6-22.5l-29.1-10.9L625 193c8.3-8.3 9.4-21.5 2.5-31s-19.7-12.7-30.2-7.4L504 201.1C483.7 132.9 426.5 80.6 355.5 67.3L492.3 340.8zm-101 93.5L220.1 92c-20.3 12.4-38.2 28.5-52.6 47.3L321.9 448c24.5-.2 47.9-5.1 69.4-13.7zM284.5 444.7L147.7 171.2c-4.7 9.6-8.7 19.6-11.8 30L42.7 154.5c-10.5-5.3-23.3-2.1-30.2 7.4s-5.8 22.7 2.5 31l29.6 29.6L15.6 233.5C6.2 237 0 246 0 256s6.2 19 15.6 22.5l29.1 10.9L15 319c-8.3 8.3-9.4 21.5-2.5 31s19.7 12.7 30.2 7.4L136 310.9c20.3 68.2 77.5 120.6 148.5 133.9z";
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
    exports.faCandy = exports.definition;
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

// src/_entries/faCandy.js
var import_faCandy = __toESM(require_faCandy());
var export_faCandy = import_faCandy.definition;
export {
  export_faCandy as faCandy
};
