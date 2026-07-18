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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faObjectSubtract.js
var require_faObjectSubtract = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faObjectSubtract.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "object-subtract";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e49e";
    var svgPathData = "M512 448c0 35.3-28.7 64-64 64l-224 0c-35.3 0-64-28.7-64-64l0-96-96 0c-35.3 0-64-28.7-64-64L0 64C0 28.7 28.7 0 64 0L288 0c35.3 0 64 28.7 64 64l0 96 96 0c35.3 0 64 28.7 64 64l0 224zM288 288l0-224L64 64l0 224 224 0z";
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
    exports.faObjectSubtract = exports.definition;
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

// src/_entries/faObjectSubtract.js
var import_faObjectSubtract = __toESM(require_faObjectSubtract());
var export_faObjectSubtract = import_faObjectSubtract.definition;
export {
  export_faObjectSubtract as faObjectSubtract
};
