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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDiagramSankey.js
var require_faDiagramSankey = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDiagramSankey.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "diagram-sankey";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e158";
    var svgPathData = "M294.4 44.8c6-8.1 15.5-12.8 25.6-12.8l224 0c17.7 0 32 14.3 32 32s-14.3 32-32 32L336 96 249.6 211.2c-6 8.1-15.5 12.8-25.6 12.8L32 224c-17.7 0-32-14.3-32-32s14.3-32 32-32l176 0L294.4 44.8zM141.9 320L32 320c-17.7 0-32-14.3-32-32s14.3-32 32-32l512 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-327.5 0 57.6 96L544 416c17.7 0 32 14.3 32 32s-14.3 32-32 32l-288 0c-11.2 0-21.7-5.9-27.4-15.5L141.9 320z";
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
    exports.faDiagramSankey = exports.definition;
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

// src/_entries/faDiagramSankey.js
var import_faDiagramSankey = __toESM(require_faDiagramSankey());
var export_faDiagramSankey = import_faDiagramSankey.definition;
export {
  export_faDiagramSankey as faDiagramSankey
};
