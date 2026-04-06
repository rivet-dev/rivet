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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSquareParkingSlash.js
var require_faSquareParkingSlash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSquareParkingSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "square-parking-slash";
    var width = 640;
    var height = 512;
    var aliases = ["parking-slash"];
    var unicode = "f617";
    var svgPathData = "M38.8 5.1C28.4-3.1 13.3-1.2 5.1 9.2S-1.2 34.7 9.2 42.9l592 464c10.4 8.2 25.5 6.3 33.7-4.1s6.3-25.5-4.1-33.7l-86.8-68L544 96c0-35.3-28.7-64-64-64L160 32c-21.6 0-40.7 10.7-52.3 27.1L38.8 5.1zM227 152.7c6-14.5 20.3-24.7 37-24.7l72 0c53 0 96 43 96 96c0 26.3-10.6 50.2-27.8 67.5l-51.4-40.3c9.1-5.6 15.2-15.7 15.2-27.2c0-17.7-14.3-32-32-32l-48 0 0 8.4-61-47.8zm259.9 327L288 322.9l0 29.1c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-64 0-15.5L96 171.6 96 416c0 35.3 28.7 64 64 64l320 0c2.3 0 4.6-.1 6.9-.4z";
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
    exports.faSquareParkingSlash = exports.definition;
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

// src/_entries/faParkingSlash.js
var import_faSquareParkingSlash = __toESM(require_faSquareParkingSlash());
var export_faParkingSlash = import_faSquareParkingSlash.definition;
export {
  export_faParkingSlash as faParkingSlash
};
