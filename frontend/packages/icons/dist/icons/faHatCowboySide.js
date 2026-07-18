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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHatCowboySide.js
var require_faHatCowboySide = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHatCowboySide.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hat-cowboy-side";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f8c1";
    var svgPathData = "M640 388.3c0 16.9-7.1 32.2-18.4 43.1l-35-23.3-292.7-195.1c-36.1-24.1-78.6-36.9-122-36.9l-3.9 0c-2.7 0-5.4 0-8 .1l22.2-100c5.7-25.8 28.6-44.1 55-44.1 12.2 0 24.1 4 33.8 11.3l4.7 3.5c26.3 19.7 62.4 19.7 88.6 0l4.7-3.5c9.8-7.3 21.6-11.3 33.8-11.3 26.4 0 49.3 18.3 55 44.1l33 148.5C574.5 232.3 640 302.6 640 388.3zM171.9 224c33.9 0 67.1 10 95.4 28.9L560 448 56 448c-30.9 0-56-25.1-56-56 0-92.8 75.2-168 168-168l3.9 0z";
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
    exports.faHatCowboySide = exports.definition;
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

// src/_entries/faHatCowboySide.js
var import_faHatCowboySide = __toESM(require_faHatCowboySide());
var export_faHatCowboySide = import_faHatCowboySide.definition;
export {
  export_faHatCowboySide as faHatCowboySide
};
