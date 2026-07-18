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

// src/node_modules/@fortawesome/free-solid-svg-icons/faCableCar.js
var require_faCableCar = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faCableCar.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "cable-car";
    var width = 512;
    var height = 512;
    var aliases = [128673, 57551, "tram"];
    var unicode = "f7da";
    var svgPathData = "M288-32a32 32 0 1 1 0 64 32 32 0 1 1 0-64zM160 24a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zM32 288c0-35.3 28.7-64 64-64l136 0 0-96.2-193.4 55.3c-12.7 3.6-26-3.7-29.7-16.5s3.7-26 16.5-29.7l224-64 0 0 224-64c12.7-3.6 26 3.7 29.7 16.5s-3.7 26-16.5 29.7l-206.6 59 0 109.9 136 0c35.3 0 64 28.7 64 64l0 160c0 35.3-28.7 64-64 64L96 512c-35.3 0-64-28.7-64-64l0-160zm80 0c-8.8 0-16 7.2-16 16l0 64c0 8.8 7.2 16 16 16l56 0 0-96-56 0zm184 96l0-96-80 0 0 96 80 0zm48 0l56 0c8.8 0 16-7.2 16-16l0-64c0-8.8-7.2-16-16-16l-56 0 0 96z";
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
    exports.faCableCar = exports.definition;
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

// src/_entries/faTram.js
var import_faCableCar = __toESM(require_faCableCar());
var export_faTram = import_faCableCar.definition;
export {
  export_faTram as faTram
};
