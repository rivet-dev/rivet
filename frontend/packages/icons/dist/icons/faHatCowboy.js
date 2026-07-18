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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHatCowboy.js
var require_faHatCowboy = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHatCowboy.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hat-cowboy";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f8c0";
    var svgPathData = "M182.2 76.1L130.8 307.5C145.5 324.9 167.4 336 192 336l256 0c24.6 0 46.5-11.1 61.2-28.5L457.8 76.1c-5.7-25.8-28.6-44.1-55-44.1-12.2 0-24.1 4-33.8 11.3l-4.7 3.5c-26.3 19.7-62.4 19.7-88.6 0L271 43.3c-9.8-7.3-21.6-11.3-33.8-11.3-26.4 0-49.3 18.3-55 44.1zM64 256c0-17.7-14.3-32-32-32S0 238.3 0 256C0 362 86 448 192 448l256 0c106 0 192-86 192-192 0-17.7-14.3-32-32-32s-32 14.3-32 32c0 70.7-57.3 128-128 128l-256 0c-70.7 0-128-57.3-128-128z";
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
    exports.faHatCowboy = exports.definition;
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

// src/_entries/faHatCowboy.js
var import_faHatCowboy = __toESM(require_faHatCowboy());
var export_faHatCowboy = import_faHatCowboy.definition;
export {
  export_faHatCowboy as faHatCowboy
};
