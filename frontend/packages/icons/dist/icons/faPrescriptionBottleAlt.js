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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPrescriptionBottleMedical.js
var require_faPrescriptionBottleMedical = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPrescriptionBottleMedical.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "prescription-bottle-medical";
    var width = 384;
    var height = 512;
    var aliases = ["prescription-bottle-alt"];
    var unicode = "f486";
    var svgPathData = "M0 32C0 14.3 14.3 0 32 0L352 0c17.7 0 32 14.3 32 32l0 32c0 17.7-14.3 32-32 32L32 96C14.3 96 0 81.7 0 64L0 32zM32 144l320 0 0 304c0 35.3-28.7 64-64 64L96 512c-35.3 0-64-28.7-64-64l0-304zM160 248l0 40-40 0c-8.8 0-16 7.2-16 16l0 32c0 8.8 7.2 16 16 16l40 0 0 40c0 8.8 7.2 16 16 16l32 0c8.8 0 16-7.2 16-16l0-40 40 0c8.8 0 16-7.2 16-16l0-32c0-8.8-7.2-16-16-16l-40 0 0-40c0-8.8-7.2-16-16-16l-32 0c-8.8 0-16 7.2-16 16z";
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
    exports.faPrescriptionBottleMedical = exports.definition;
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

// src/_entries/faPrescriptionBottleAlt.js
var import_faPrescriptionBottleMedical = __toESM(require_faPrescriptionBottleMedical());
var export_faPrescriptionBottleAlt = import_faPrescriptionBottleMedical.definition;
export {
  export_faPrescriptionBottleAlt as faPrescriptionBottleAlt
};
