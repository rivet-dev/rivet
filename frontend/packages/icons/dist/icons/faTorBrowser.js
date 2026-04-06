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

// src/node_modules/@fortawesome/free-brands-svg-icons/faTorBrowser.js
var require_faTorBrowser = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faTorBrowser.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "tor-browser";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e838";
    var svgPathData = "M256.5 465.4l0-31c98.3-.3 177.9-80 177.9-178.4S354.8 77.9 256.5 77.6l0-31C372 46.8 465.4 140.5 465.4 256S372 465.2 256.5 465.4zm0-108.6c55.4-.3 100.3-45.3 100.3-100.8S312 155.5 256.5 155.2l0-31c72.6 .3 131.4 59.2 131.4 131.8S329.1 387.6 256.5 387.8l0-31zm0-155.1c29.7 .3 53.8 24.5 53.8 54.3s-24 54-53.8 54.3l0-108.6zM0 256a256 256 0 1 0 512 0 256 256 0 1 0 -512 0z";
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
    exports.faTorBrowser = exports.definition;
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

// src/_entries/faTorBrowser.js
var import_faTorBrowser = __toESM(require_faTorBrowser());
var export_faTorBrowser = import_faTorBrowser.definition;
export {
  export_faTorBrowser as faTorBrowser
};
