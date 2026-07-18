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

// src/node_modules/@fortawesome/free-solid-svg-icons/faMicrophoneSlash.js
var require_faMicrophoneSlash = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faMicrophoneSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "microphone-slash";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f131";
    var svgPathData = "M41-24.9c-9.4-9.4-24.6-9.4-33.9 0S-2.3-.3 7 9.1l528 528c9.4 9.4 24.6 9.4 33.9 0s9.4-24.6 0-33.9L424.7 358.8C458.9 324.2 480 276.6 480 224l0-40c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 40c0 39.3-15.7 74.9-41.3 100.9L356.8 291C373.6 273.7 384 250 384 224l0-128c0-53-43-96-96-96s-96 43-96 96l0 30.2-151-151zm298.3 434l-41.4-41.4c-3.3 .2-6.5 .3-9.8 .3-79.5 0-144-64.5-144-144l0-10.2-43.6-43.6c-2.8 3.9-4.4 8.7-4.4 13.8l0 40c0 97.9 73.3 178.7 168 190.5l0 49.5-48 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l144 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-48 0 0-49.5c9.3-1.2 18.4-3 27.3-5.4z";
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
    exports.faMicrophoneSlash = exports.definition;
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

// src/_entries/faMicrophoneSlash.js
var import_faMicrophoneSlash = __toESM(require_faMicrophoneSlash());
var export_faMicrophoneSlash = import_faMicrophoneSlash.definition;
export {
  export_faMicrophoneSlash as faMicrophoneSlash
};
