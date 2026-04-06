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

// src/node_modules/@fortawesome/free-solid-svg-icons/faLaptopMedical.js
var require_faLaptopMedical = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faLaptopMedical.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "laptop-medical";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f812";
    var svgPathData = "M64 96c0-35.3 28.7-64 64-64l384 0c35.3 0 64 28.7 64 64l0 240-64 0 0-240-384 0 0 240-64 0 0-240zM0 403.2C0 392.6 8.6 384 19.2 384l601.6 0c10.6 0 19.2 8.6 19.2 19.2 0 42.4-34.4 76.8-76.8 76.8L76.8 480C34.4 480 0 445.6 0 403.2zM288 160c0-8.8 7.2-16 16-16l32 0c8.8 0 16 7.2 16 16l0 40 40 0c8.8 0 16 7.2 16 16l0 32c0 8.8-7.2 16-16 16l-40 0 0 40c0 8.8-7.2 16-16 16l-32 0c-8.8 0-16-7.2-16-16l0-40-40 0c-8.8 0-16-7.2-16-16l0-32c0-8.8 7.2-16 16-16l40 0 0-40z";
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
    exports.faLaptopMedical = exports.definition;
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

// src/_entries/faLaptopMedical.js
var import_faLaptopMedical = __toESM(require_faLaptopMedical());
var export_faLaptopMedical = import_faLaptopMedical.definition;
export {
  export_faLaptopMedical as faLaptopMedical
};
