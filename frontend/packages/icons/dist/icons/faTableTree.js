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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTableTree.js
var require_faTableTree = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTableTree.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "table-tree";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e293";
    var svgPathData = "M64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l384 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64L64 32zM448 160l0 256L64 416l0-256 32 0 0 64 0 96c0 26.5 21.5 48 48 48l96 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-96 0c-8.8 0-16-7.2-16-16l0-50.7c5 1.8 10.4 2.7 16 2.7l32 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-32 0c-8.8 0-16-7.2-16-16l0-64 320 0zM224 240l0 32c0 8.8 7.2 16 16 16l160 0c8.8 0 16-7.2 16-16l0-32c0-8.8-7.2-16-16-16l-160 0c-8.8 0-16 7.2-16 16zm80 80c-8.8 0-16 7.2-16 16l0 32c0 8.8 7.2 16 16 16l96 0c8.8 0 16-7.2 16-16l0-32c0-8.8-7.2-16-16-16l-96 0z";
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
    exports.faTableTree = exports.definition;
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

// src/_entries/faTableTree.js
var import_faTableTree = __toESM(require_faTableTree());
var export_faTableTree = import_faTableTree.definition;
export {
  export_faTableTree as faTableTree
};
