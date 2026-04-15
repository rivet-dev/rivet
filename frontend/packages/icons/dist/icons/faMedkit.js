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

// src/node_modules/@fortawesome/free-solid-svg-icons/faSuitcaseMedical.js
var require_faSuitcaseMedical = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faSuitcaseMedical.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "suitcase-medical";
    var width = 512;
    var height = 512;
    var aliases = ["medkit"];
    var unicode = "f0fa";
    var svgPathData = "M192 56l0 40 128 0 0-40c0-4.4-3.6-8-8-8L200 48c-4.4 0-8 3.6-8 8zm-48 8l0-8c0-30.9 25.1-56 56-56L312 0c30.9 0 56 25.1 56 56l0 424-224 0 0-416zM96 176l0 304-32 0c-35.3 0-64-28.7-64-64L0 160c0-35.3 28.7-64 64-64l32 0 0 80zM416 480l0-384 32 0c35.3 0 64 28.7 64 64l0 256c0 35.3-28.7 64-64 64l-32 0zM244 208c-8.8 0-16 7.2-16 16l0 36-36 0c-8.8 0-16 7.2-16 16l0 24c0 8.8 7.2 16 16 16l36 0 0 36c0 8.8 7.2 16 16 16l24 0c8.8 0 16-7.2 16-16l0-36 36 0c8.8 0 16-7.2 16-16l0-24c0-8.8-7.2-16-16-16l-36 0 0-36c0-8.8-7.2-16-16-16l-24 0z";
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
    exports.faSuitcaseMedical = exports.definition;
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

// src/_entries/faMedkit.js
var import_faSuitcaseMedical = __toESM(require_faSuitcaseMedical());
var export_faMedkit = import_faSuitcaseMedical.definition;
export {
  export_faMedkit as faMedkit
};
