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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faWindWarning.js
var require_faWindWarning = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faWindWarning.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "wind-warning";
    var width = 640;
    var height = 512;
    var aliases = ["wind-circle-exclamation"];
    var unicode = "f776";
    var svgPathData = "M418 368c-38.7 67-111.1 112-194 112C100.3 480 0 379.7 0 256S100.3 32 224 32c118.3 0 215.2 91.8 223.4 208L384 240c-35.3 0-64 28.7-64 64s28.7 64 64 64l34 0zM248 152c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 112c0 13.3 10.7 24 24 24s24-10.7 24-24l0-112zM224 384a32 32 0 1 0 0-64 32 32 0 1 0 0 64zm248 48c0-17.7 14.3-32 32-32l32 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-152 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l152 0c53 0 96 43 96 96s-43 96-96 96l-32 0c-17.7 0-32-14.3-32-32zm72-192l-64.5 0c-1.4-22.2-5.6-43.7-12.3-64l76.7 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-32 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l32 0c53 0 96 43 96 96s-43 96-96 96z";
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
    exports.faWindWarning = exports.definition;
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

// src/_entries/faWindCircleExclamation.js
var import_faWindWarning = __toESM(require_faWindWarning());
var export_faWindCircleExclamation = import_faWindWarning.definition;
export {
  export_faWindCircleExclamation as faWindCircleExclamation
};
