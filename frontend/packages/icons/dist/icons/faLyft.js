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

// src/node_modules/@fortawesome/free-brands-svg-icons/faLyft.js
var require_faLyft = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faLyft.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "lyft";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f3c3";
    var svgPathData = "M0 81.1l77.8 0 0 208.7c0 33.1 15 52.8 27.2 61-12.7 11.1-51.2 20.9-80.2-2.8-17-14-24.8-37.3-24.8-59L0 81.1zM485.9 254.6l0-22 23.8 0 0-76.8-26.1 0c-10.1-46.3-51.2-80.7-100.3-80.7-56.6 0-102.7 46-102.7 102.7l0 179.2c16 2.3 35.4-.3 51.7-14 17.1-14 24.8-37.2 24.8-59l0-6.7 38.8 0 0-76.8-38.8 0 0-23.3c0-34.6 52.2-34.6 52.2 0l0 77.1c0 56.6 46 102.7 102.7 102.7l0-76.5c-14.5 0-26.1-11.7-26.1-25.9zm-294.3-99l0 113c0 15.4-23.8 15.4-23.8 0l0-113-76.8 0 0 132.7c0 23.8 8 54 45 63.9 37 9.8 58.2-10.6 58.2-10.6-2.1 13.4-14.5 23.3-34.9 25.3-15.5 1.6-35.2-3.6-45-7.8l0 70.3c25.1 7.5 51.5 9.8 77.6 4.7 47.1-9.1 76.8-48.4 76.8-100.8l0-178.2-77.1 0 0 .5z";
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
    exports.faLyft = exports.definition;
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

// src/_entries/faLyft.js
var import_faLyft = __toESM(require_faLyft());
var export_faLyft = import_faLyft.definition;
export {
  export_faLyft as faLyft
};
