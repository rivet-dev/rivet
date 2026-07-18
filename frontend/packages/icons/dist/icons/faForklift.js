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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faForklift.js
var require_faForklift = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faForklift.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "forklift";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f47a";
    var svgPathData = "M48 160l48 0L96 48c0-26.5 21.5-48 48-48L292.7 0c20.3 0 38.4 12.8 45.2 31.9l75.3 210.8c1.8 5.2 2.8 10.6 2.8 16.1l0 85.7c19.6 17.6 32 43.1 32 71.6c0 53-43 96-96 96s-96-43-96-96l-64 0c0 53-43 96-96 96s-96-43-96-96L0 208c0-26.5 21.5-48 48-48zm208 96l94 0L281.4 64 160 64l0 96 96 96zM96 464a48 48 0 1 0 0-96 48 48 0 1 0 0 96zm256 0a48 48 0 1 0 0-96 48 48 0 1 0 0 96zM544 32l0 352 64 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-96 0c-17.7 0-32-14.3-32-32l0-384c0-17.7 14.3-32 32-32s32 14.3 32 32z";
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
    exports.faForklift = exports.definition;
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

// src/_entries/faForklift.js
var import_faForklift = __toESM(require_faForklift());
var export_faForklift = import_faForklift.definition;
export {
  export_faForklift as faForklift
};
