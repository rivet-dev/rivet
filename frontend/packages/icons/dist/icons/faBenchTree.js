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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBenchTree.js
var require_faBenchTree = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBenchTree.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bench-tree";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e2e7";
    var svgPathData = "M608 96c0 5.1-.4 10.2-1.2 15.1C626.9 125.7 640 149.3 640 176c0 44.2-35.8 80-80 80l-16 0 0 224c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-224-16 0c-44.2 0-80-35.8-80-80c0-26.7 13.1-50.3 33.2-64.9c-.8-4.9-1.2-10-1.2-15.1c0-53 43-96 96-96s96 43 96 96zM32 224c0-17.7 14.3-32 32-32l256 0c17.7 0 32 14.3 32 32l0 64c0 17.7-14.3 32-32 32L64 320c-17.7 0-32-14.3-32-32l0-64zm0 128l320 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l0 64c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-64L96 416l0 64c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-64c-17.7 0-32-14.3-32-32s14.3-32 32-32z";
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
    exports.faBenchTree = exports.definition;
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

// src/_entries/faBenchTree.js
var import_faBenchTree = __toESM(require_faBenchTree());
var export_faBenchTree = import_faBenchTree.definition;
export {
  export_faBenchTree as faBenchTree
};
