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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPrescriptionBottle.js
var require_faPrescriptionBottle = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPrescriptionBottle.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "prescription-bottle";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "f485";
    var svgPathData = "M0 32C0 14.3 14.3 0 32 0L352 0c17.7 0 32 14.3 32 32l0 32c0 17.7-14.3 32-32 32L32 96C14.3 96 0 81.7 0 64L0 32zM32 144l320 0 0 304c0 35.3-28.7 64-64 64L96 512c-35.3 0-64-28.7-64-64l72 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-72 0 0-48 72 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-72 0 0-48 72 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-72 0 0-64z";
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
    exports.faPrescriptionBottle = exports.definition;
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

// src/_entries/faPrescriptionBottle.js
var import_faPrescriptionBottle = __toESM(require_faPrescriptionBottle());
var export_faPrescriptionBottle = import_faPrescriptionBottle.definition;
export {
  export_faPrescriptionBottle as faPrescriptionBottle
};
