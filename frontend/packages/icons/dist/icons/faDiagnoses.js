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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPersonDotsFromLine.js
var require_faPersonDotsFromLine = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPersonDotsFromLine.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-dots-from-line";
    var width = 512;
    var height = 512;
    var aliases = ["diagnoses"];
    var unicode = "f470";
    var svgPathData = "M184 72a72 72 0 1 1 144 0 72 72 0 1 1 -144 0zM160 299.3c-19.9 22.6-32 52.2-32 84.7l0 32-64 0 0-32c0-106 86-192 192-192s192 86 192 192l0 32-64 0 0-32c0-32.5-12.1-62.1-32-84.7l0 116.7-192 0 0-116.7zM232 384a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm88-104a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zM24 464l464 0c13.3 0 24 10.7 24 24s-10.7 24-24 24L24 512c-13.3 0-24-10.7-24-24s10.7-24 24-24zM64 184a24 24 0 1 1 48 0 24 24 0 1 1 -48 0zm424 72a24 24 0 1 1 0 48 24 24 0 1 1 0-48z";
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
    exports.faPersonDotsFromLine = exports.definition;
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

// src/_entries/faDiagnoses.js
var import_faPersonDotsFromLine = __toESM(require_faPersonDotsFromLine());
var export_faDiagnoses = import_faPersonDotsFromLine.definition;
export {
  export_faDiagnoses as faDiagnoses
};
