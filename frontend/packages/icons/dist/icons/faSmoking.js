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

// src/node_modules/@fortawesome/free-solid-svg-icons/faSmoking.js
var require_faSmoking = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faSmoking.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "smoking";
    var width = 576;
    var height = 512;
    var aliases = [128684];
    var unicode = "f48d";
    var svgPathData = "M360 0c13.3 0 24 10.7 24 24l0 19c0 40.3 16 79 44.5 107.5l21 21C469 191 480 217.4 480 245l0 19c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-19c0-14.9-5.9-29.1-16.4-39.6l-21-21C357.1 146.9 336 96 336 43l0-19c0-13.3 10.7-24 24-24zM552 288c-13.3 0-24-10.7-24-24l0-19c0-40.3-16-79-44.5-107.5l-21-21C443 97 432 70.6 432 43l0-19c0-13.3 10.7-24 24-24s24 10.7 24 24l0 19c0 14.9 5.9 29.1 16.4 39.6l21 21C554.9 141.1 576 192 576 245l0 19c0 13.3-10.7 24-24 24zM320 448l96 0 0-32-96 0 0 32zM0 416c0-35.3 28.7-64 64-64l384 0c17.7 0 32 14.3 32 32l0 96c0 17.7-14.3 32-32 32L64 512c-35.3 0-64-28.7-64-64l0-32zm576-40l0 112c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-112c0-13.3 10.7-24 24-24s24 10.7 24 24z";
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
    exports.faSmoking = exports.definition;
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

// src/_entries/faSmoking.js
var import_faSmoking = __toESM(require_faSmoking());
var export_faSmoking = import_faSmoking.definition;
export {
  export_faSmoking as faSmoking
};
