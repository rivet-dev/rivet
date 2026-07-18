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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCupTogo.js
var require_faCupTogo = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCupTogo.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "cup-togo";
    var width = 448;
    var height = 512;
    var aliases = ["coffee-togo"];
    var unicode = "f6c5";
    var svgPathData = "M55.2 17.7C60.6 6.8 71.7 0 83.8 0L364.2 0c12.1 0 23.2 6.8 28.6 17.7L416 64c17.7 0 32 14.3 32 32s-14.3 32-32 32L32 128C14.3 128 0 113.7 0 96S14.3 64 32 64L55.2 17.7zM74.7 453.8L48 160l352 0L373.3 453.8c-3 33-30.6 58.2-63.7 58.2l-171.1 0c-33.1 0-60.7-25.2-63.7-58.2zM323.8 290.9c1.7-18.7-13.1-34.9-31.9-34.9L156 256c-18.8 0-33.6 16.2-31.9 34.9l8.7 96c1.5 16.5 15.3 29.1 31.9 29.1l118.5 0c16.6 0 30.4-12.6 31.9-29.1l8.7-96z";
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
    exports.faCupTogo = exports.definition;
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

// src/_entries/faCoffeeTogo.js
var import_faCupTogo = __toESM(require_faCupTogo());
var export_faCoffeeTogo = import_faCupTogo.definition;
export {
  export_faCoffeeTogo as faCoffeeTogo
};
