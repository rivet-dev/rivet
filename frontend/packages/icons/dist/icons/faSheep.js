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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSheep.js
var require_faSheep = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSheep.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sheep";
    var width = 640;
    var height = 512;
    var aliases = [128017];
    var unicode = "f711";
    var svgPathData = "M384 135.8l0 72.2c0 44.2 35.8 80 80 80l32 0c44.2 0 80-35.8 80-80l0-72.2 14.5 6.2c12.2 5.2 26.3-.4 31.5-12.6s-.4-26.3-12.6-31.5L561.6 77.4C544.7 50.2 514.5 32 480 32s-64.7 18.2-81.6 45.4L350.5 97.9c-12.2 5.2-17.8 19.3-12.6 31.5s19.3 17.8 31.5 12.6l14.5-6.2zm80-7.8a16 16 0 1 1 0 32 16 16 0 1 1 0-32zm48 16a16 16 0 1 1 32 0 16 16 0 1 1 -32 0zM352 175.4c-18.7-2.7-35.5-14.8-43.5-33.4c-6.9-16.1-5.7-33.5 1.8-47.9C299 76 278.9 64 256 64c-19.2 0-36.5 8.5-48.2 21.9C196.9 77.2 183 72 168 72c-30.1 0-55.3 20.8-62.2 48.8c-3.2-.5-6.5-.8-9.8-.8c-35.3 0-64 28.7-64 64c0 5.3 .7 10.5 1.9 15.5C13.7 210.3 0 231.5 0 256s13.7 45.7 33.9 56.5c-1.2 5-1.9 10.2-1.9 15.5c0 35.3 28.7 64 64 64c.5 0 1 0 1.5 0l17.6 93.9c2.8 15.1 16.1 26.1 31.5 26.1l29.4 0c17.7 0 32-14.3 32-32l0-53.7c11.7 13.3 28.9 21.7 48 21.7s36.3-8.4 48-21.7l0 53.7c0 17.7 14.3 32 32 32l29.4 0c15.4 0 28.6-11 31.4-26.1L414.5 392c.5 0 1 0 1.5 0c35.3 0 64-28.7 64-64c0-2.7-.2-5.4-.5-8L464 320c-61.9 0-112-50.1-112-112l0-32.6z";
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
    exports.faSheep = exports.definition;
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

// src/_entries/faSheep.js
var import_faSheep = __toESM(require_faSheep());
var export_faSheep = import_faSheep.definition;
export {
  export_faSheep as faSheep
};
