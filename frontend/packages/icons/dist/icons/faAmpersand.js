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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faAmpersand.js
var require_faAmpersand = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faAmpersand.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ampersand";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "26";
    var svgPathData = "M180.7 96l38 0c20.6 0 37.3 16.7 37.3 37.3c0 11.8-5.5 22.8-14.9 29.9l-48 36-39-41.3c-6.4-6.8-10-15.8-10-25.2c0-20.3 16.4-36.7 36.7-36.7zm56.6 150l42.2-31.6C305 195.3 320 165.2 320 133.3C320 77.4 274.6 32 218.7 32l-38 0C125.1 32 80 77.1 80 132.7c0 25.7 9.8 50.4 27.5 69.1l34 36L74.7 288C47.8 308.1 32 339.8 32 373.3C32 432.2 79.8 480 138.7 480l74.5 0c37.4 0 72.4-18.7 93.2-49.9l6.9-10.4L360.7 470c12.1 12.8 32.4 13.4 45.2 1.3s13.4-32.4 1.3-45.2l-57.5-60.9 60.9-91.4c9.8-14.7 5.8-34.6-8.9-44.4s-34.6-5.8-44.4 8.9l-52.7 79.1L237.3 246zm-51.5 38.6L268.2 372l-15.1 22.6C244.2 408 229.2 416 213.2 416l-74.5 0C115.1 416 96 396.9 96 373.3c0-13.4 6.3-26.1 17.1-34.1l72.7-54.5z";
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
    exports.faAmpersand = exports.definition;
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

// src/_entries/faAmpersand.js
var import_faAmpersand = __toESM(require_faAmpersand());
var export_faAmpersand = import_faAmpersand.definition;
export {
  export_faAmpersand as faAmpersand
};
