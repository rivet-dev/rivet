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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCustard.js
var require_faCustard = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCustard.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "custard";
    var width = 576;
    var height = 512;
    var aliases = [127854];
    var unicode = "e403";
    var svgPathData = "M396.7 64c30 0 56 20.8 62.5 50.1l13.5 60.8C450.9 167.4 423.5 160 400 160c-26.4 0-45.8 9.4-62.2 17.2l-.7 .3c-16.7 8-30.4 14.4-49.1 14.4s-32.3-6.4-49.1-14.4l-.7-.3C221.8 169.4 202.4 160 176 160c-23.5 0-50.9 7.4-72.6 14.9l13.5-60.8C123.4 84.8 149.3 64 179.3 64l217.3 0zM64.3 350.9L95.1 212.1c4.1-1.6 8.5-3.3 13.2-5c22-7.9 47.8-15.1 67.7-15.1c18.7 0 32.3 6.4 49.1 14.4l.7 .3c16.4 7.9 35.8 17.2 62.2 17.2s45.8-9.4 62.2-17.2l.7-.3c16.7-8 30.4-14.4 49.1-14.4c19.8 0 45.6 7.2 67.7 15.1c4.7 1.7 9.2 3.4 13.2 5L512 352 64.3 350.9zM32 384l512 0c17.7 0 32 14.3 32 32s-14.3 32-32 32L32 448c-17.7 0-32-14.3-32-32s14.3-32 32-32z";
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
    exports.faCustard = exports.definition;
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

// src/_entries/faCustard.js
var import_faCustard = __toESM(require_faCustard());
var export_faCustard = import_faCustard.definition;
export {
  export_faCustard as faCustard
};
