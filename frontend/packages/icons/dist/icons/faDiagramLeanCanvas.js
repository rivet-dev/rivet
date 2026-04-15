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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDiagramLeanCanvas.js
var require_faDiagramLeanCanvas = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDiagramLeanCanvas.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "diagram-lean-canvas";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e156";
    var svgPathData = "M512 96l0 208 64 0 0-208-64 0zm-48 0l-64 0 0 80 64 0 0-80zM352 96l-64 0 0 208 64 0 0-208zM240 96l-64 0 0 80 64 0 0-80zM128 96L64 96l0 208 64 0 0-208zM64 352l0 64 232 0 0-64L64 352zm280 64l232 0 0-64-232 0 0 64zM0 96C0 60.7 28.7 32 64 32l512 0c35.3 0 64 28.7 64 64l0 320c0 35.3-28.7 64-64 64L64 480c-35.3 0-64-28.7-64-64L0 96zM240 304l0-80-64 0 0 80 64 0zm224-80l-64 0 0 80 64 0 0-80z";
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
    exports.faDiagramLeanCanvas = exports.definition;
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

// src/_entries/faDiagramLeanCanvas.js
var import_faDiagramLeanCanvas = __toESM(require_faDiagramLeanCanvas());
var export_faDiagramLeanCanvas = import_faDiagramLeanCanvas.definition;
export {
  export_faDiagramLeanCanvas as faDiagramLeanCanvas
};
