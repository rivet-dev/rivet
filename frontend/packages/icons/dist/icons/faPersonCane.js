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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPersonCane.js
var require_faPersonCane = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPersonCane.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-cane";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e53c";
    var svgPathData = "M232-32a56 56 0 1 1 0 112 56 56 0 1 1 0-112zM144 236.7L97.7 299.1c-10.5 14.2-30.6 17.2-44.8 6.6s-17.2-30.6-6.6-44.8l70.5-95C142 132 181.7 112 224 112s82 20 107.2 53.9l70.5 95c10.5 14.2 7.6 34.2-6.6 44.8s-34.2 7.6-44.8-6.6L304 236.7 304 512c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-160c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 160c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-275.3zM392 384c-4.4 0-8 3.6-8 8 0 13.3-10.7 24-24 24s-24-10.7-24-24c0-30.9 25.1-56 56-56s56 25.1 56 56l0 128c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-128c0-4.4-3.6-8-8-8z";
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
    exports.faPersonCane = exports.definition;
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

// src/_entries/faPersonCane.js
var import_faPersonCane = __toESM(require_faPersonCane());
var export_faPersonCane = import_faPersonCane.definition;
export {
  export_faPersonCane as faPersonCane
};
