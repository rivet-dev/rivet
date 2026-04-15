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

// src/node_modules/@fortawesome/free-solid-svg-icons/faRibbon.js
var require_faRibbon = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faRibbon.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ribbon";
    var width = 384;
    var height = 512;
    var aliases = [127895];
    var unicode = "f4d6";
    var svgPathData = "M235.1 0c33.4 0 64.5 17.4 81.9 45.9 1.2 2 13 21.3 35.3 57.8 21.1 34.5 18.3 78.5-7 110L278.3 297.7 364.5 406c5.5 6.9 4.4 16.9-2.5 22.5l-80 64c-6.9 5.5-17 4.4-22.5-2.5L38.6 213.8C13.3 182.3 10.5 138.3 31.6 103.8 54 67.2 65.7 47.9 67 45.9 84.4 17.4 115.4 0 148.9 0l86.3 0zM192 189.2l48.6-61.2-97.3 0 48.6 61.2zM75 336.2l86.2 107.8-36.8 46c-5.5 6.9-15.6 8-22.5 2.5l-80-64c-6.9-5.5-8-15.6-2.5-22.5L75 336.2z";
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
    exports.faRibbon = exports.definition;
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

// src/_entries/faRibbon.js
var import_faRibbon = __toESM(require_faRibbon());
var export_faRibbon = import_faRibbon.definition;
export {
  export_faRibbon as faRibbon
};
