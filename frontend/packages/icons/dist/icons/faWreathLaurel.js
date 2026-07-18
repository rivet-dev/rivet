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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faWreathLaurel.js
var require_faWreathLaurel = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faWreathLaurel.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "wreath-laurel";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e5d2";
    var svgPathData = "M144 112C144 50.1 194.1 0 256 0l16 0c8.8 0 16 7.2 16 16l0 16c0 60.2-47.5 109.3-107 111.9c-23.9 32.2-37 71.5-37 112.1c0 97.2 78.8 176 176 176l11.6 0c43.5 0 86.3 10.1 125.2 29.6l9.9 5c11.9 5.9 16.7 20.3 10.7 32.2s-20.3 16.7-32.2 10.7l-9.9-5C403.1 488.4 367.6 480 331.6 480L320 480l-11.6 0c-36 0-71.5 8.4-103.8 24.5l-9.9 5c-11.9 5.9-26.3 1.1-32.2-10.7s-1.1-26.3 10.7-32.2l9.9-5c8.3-4.2 16.8-7.9 25.4-11.2c-5.6-3.2-11-6.7-16.3-10.3c-59.6 28.2-131.9 6.3-165.4-51.8l-8-13.9c-4.4-7.7-1.8-17.4 5.9-21.9c13.8-8 28.4-13.1 43.3-15.5C27.5 315.6 0 273 0 224l0-16c0-8.8 7.2-16 16-16c15.9 0 31.1 2.9 45.2 8.2c-24.3-38.8-26.8-89.5-2.3-131.9l8-13.9c4.4-7.7 14.2-10.3 21.9-5.9C115 63.7 133.8 86.8 144 113l0-1zm315 31.9C399.5 141.3 352 92.2 352 32l0-16c0-8.8 7.2-16 16-16l16 0c61.9 0 112 50.1 112 112l0 1c10.2-26.2 29-49.2 55.3-64.4c7.7-4.4 17.4-1.8 21.9 5.9l8 13.9c24.5 42.4 21.9 93.1-2.3 131.9c14.1-5.3 29.3-8.2 45.2-8.2c8.8 0 16 7.2 16 16l0 16c0 49-27.5 91.6-68 113.1c14.8 2.4 29.5 7.5 43.3 15.5c7.7 4.4 10.3 14.2 5.9 21.9l-8 13.9c-24.2 42-68.7 65.1-114 64c-4.8-5.8-10.9-10.8-18.1-14.4l-9.9-5c-2.8-1.4-5.7-2.8-8.6-4.1c-10.4-4.8-21-9-31.8-12.6c-7.9-2.7-15.9-5-24-7C460 378.9 496 321.7 496 256c0-40.6-13.1-79.9-37-112.1z";
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
    exports.faWreathLaurel = exports.definition;
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

// src/_entries/faWreathLaurel.js
var import_faWreathLaurel = __toESM(require_faWreathLaurel());
var export_faWreathLaurel = import_faWreathLaurel.definition;
export {
  export_faWreathLaurel as faWreathLaurel
};
