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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faScannerGun.js
var require_faScannerGun = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faScannerGun.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "scanner-gun";
    var width = 576;
    var height = 512;
    var aliases = ["scanner"];
    var unicode = "f488";
    var svgPathData = "M112 32C50.1 32 0 82.1 0 144c0 50.7 33.8 93.6 80 107.4L8.1 374.7c-13.3 22.8-5.7 52.2 17.1 65.6l53.4 31.4c22.8 13.4 52.3 5.8 65.7-17L261.2 256l42.8 0c26.5 0 48-21.5 48-48l0-128c0-26.5-21.5-48-48-48L112 32zm328 0c-13.3 0-24 10.7-24 24l0 16c0 13.3 10.7 24 24 24l112 0c13.3 0 24-10.7 24-24l0-16c0-13.3-10.7-24-24-24L440 32zM416 216l0 16c0 13.3 10.7 24 24 24l112 0c13.3 0 24-10.7 24-24l0-16c0-13.3-10.7-24-24-24l-112 0c-13.3 0-24 10.7-24 24zm24 200c-13.3 0-24 10.7-24 24l0 16c0 13.3 10.7 24 24 24l112 0c13.3 0 24-10.7 24-24l0-16c0-13.3-10.7-24-24-24l-112 0zM416 144c0 8.8 7.2 16 16 16l128 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-128 0c-8.8 0-16 7.2-16 16zm16 144c-8.8 0-16 7.2-16 16s7.2 16 16 16l128 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-128 0zm-16 80c0 8.8 7.2 16 16 16l128 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-128 0c-8.8 0-16 7.2-16 16z";
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
    exports.faScannerGun = exports.definition;
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

// src/_entries/faScanner.js
var import_faScannerGun = __toESM(require_faScannerGun());
var export_faScanner = import_faScannerGun.definition;
export {
  export_faScanner as faScanner
};
