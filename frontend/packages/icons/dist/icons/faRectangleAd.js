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

// src/node_modules/@fortawesome/free-solid-svg-icons/faRectangleAd.js
var require_faRectangleAd = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faRectangleAd.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "rectangle-ad";
    var width = 512;
    var height = 512;
    var aliases = ["ad"];
    var unicode = "f641";
    var svgPathData = "M64 64C28.7 64 0 92.7 0 128L0 384c0 35.3 28.7 64 64 64l384 0c35.3 0 64-28.7 64-64l0-256c0-35.3-28.7-64-64-64L64 64zM416 184l0 144c0 13.3-10.7 24-24 24-7.1 0-13.5-3.1-17.9-8-10.2 5.1-21.8 8-34.1 8-42 0-76-34-76-76s34-76 76-76c9.9 0 19.3 1.9 28 5.3l0-21.3c0-13.3 10.7-24 24-24s24 10.7 24 24zm-48 92a28 28 0 1 0 -56 0 28 28 0 1 0 56 0zM160 208c-8.8 0-16 7.2-16 16l0 32 48 0 0-32c0-8.8-7.2-16-16-16l-16 0zm32 96l-48 0 0 24c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-104c0-35.3 28.7-64 64-64l16 0c35.3 0 64 28.7 64 64l0 104c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-24z";
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
    exports.faRectangleAd = exports.definition;
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

// src/_entries/faRectangleAd.js
var import_faRectangleAd = __toESM(require_faRectangleAd());
var export_faRectangleAd = import_faRectangleAd.definition;
export {
  export_faRectangleAd as faRectangleAd
};
