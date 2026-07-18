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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPersonDressBurst.js
var require_faPersonDressBurst = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPersonDressBurst.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-dress-burst";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e544";
    var svgPathData = "M208 40.1c13.3 0 24-10.7 24-24l0-48c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 48c0 13.3 10.7 24 24 24zM8 144.1c0 13.3 10.7 24 24 24l48 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-48 0c-13.3 0-24 10.7-24 24zm92.5 141.4l33.9-33.9c9.4-9.4 9.4-24.6 0-33.9s-24.6-9.4-33.9 0L66.6 251.6c-9.4 9.4-9.4 24.6 0 33.9s24.6 9.4 33.9 0zM66.6 2.7c-9.4 9.4-9.4 24.6 0 33.9l33.9 33.9c9.4 9.4 24.6 9.4 33.9 0s9.4-24.6 0-33.9L100.5 2.7C91.1-6.7 76-6.7 66.6 2.7zM352 80a56 56 0 1 0 0-112 56 56 0 1 0 0 112zM246.2 384l25.8 0 0 128c0 17.7 14.3 32 32 32s32-14.3 32-32l0-128 32 0 0 128c0 17.7 14.3 32 32 32s32-14.3 32-32l0-128 25.8 0c10.9 0 18.6-10.7 15.2-21.1l-43-129 48.3 65.1c10.5 14.2 30.6 17.2 44.8 6.6s17.2-30.6 6.6-44.8l-70.5-95C434 132 394.3 112 352 112s-82 20-107.2 53.9l-70.5 95c-10.5 14.2-7.6 34.2 6.6 44.8s34.2 7.6 44.8-6.6l48.3-65.1-43 129c-3.5 10.4 4.3 21.1 15.2 21.1z";
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
    exports.faPersonDressBurst = exports.definition;
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

// src/_entries/faPersonDressBurst.js
var import_faPersonDressBurst = __toESM(require_faPersonDressBurst());
var export_faPersonDressBurst = import_faPersonDressBurst.definition;
export {
  export_faPersonDressBurst as faPersonDressBurst
};
