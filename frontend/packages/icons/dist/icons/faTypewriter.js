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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTypewriter.js
var require_faTypewriter = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTypewriter.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "typewriter";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f8e7";
    var svgPathData = "M64 64C64 28.7 92.7 0 128 0L354.7 0c17 0 33.3 6.7 45.3 18.7L429.3 48c12 12 18.7 28.3 18.7 45.3l0 98.7 32 0c17.7 0 32 14.3 32 32l0 32c0 17.7-14.3 32-32 32l0 160c0 35.3-28.7 64-64 64L96 512c-35.3 0-64-28.7-64-64l0-160c-17.7 0-32-14.3-32-32l0-32c0-17.7 14.3-32 32-32l32 0L64 64zm320 64l-32 0c-17.7 0-32-14.3-32-32l0-32L128 64l0 128 18.7 0c8.5 0 16.6 3.4 22.6 9.4l13.3 13.3c6 6 14.1 9.4 22.6 9.4l101.5 0c8.5 0 16.6-3.4 22.6-9.4l13.3-13.3c6-6 14.1-9.4 22.6-9.4l18.7 0 0-64zM160 288a16 16 0 1 0 0 32 16 16 0 1 0 0-32zm64 0a16 16 0 1 0 0 32 16 16 0 1 0 0-32zm64 0a16 16 0 1 0 0 32 16 16 0 1 0 0-32zm64 0a16 16 0 1 0 0 32 16 16 0 1 0 0-32zm64 0a16 16 0 1 0 0 32 16 16 0 1 0 0-32zM96 288a16 16 0 1 0 0 32 16 16 0 1 0 0-32zm64 128c-8.8 0-16 7.2-16 16s7.2 16 16 16l192 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-192 0zm-32-32a16 16 0 1 0 0-32 16 16 0 1 0 0 32zm64 0a16 16 0 1 0 0-32 16 16 0 1 0 0 32zm80-16a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zm48 16a16 16 0 1 0 0-32 16 16 0 1 0 0 32zm80-16a16 16 0 1 0 -32 0 16 16 0 1 0 32 0z";
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
    exports.faTypewriter = exports.definition;
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

// src/_entries/faTypewriter.js
var import_faTypewriter = __toESM(require_faTypewriter());
var export_faTypewriter = import_faTypewriter.definition;
export {
  export_faTypewriter as faTypewriter
};
