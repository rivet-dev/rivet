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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faIslandTropical.js
var require_faIslandTropical = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faIslandTropical.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "island-tropical";
    var width = 512;
    var height = 512;
    var aliases = [127965, "island-tree-palm"];
    var unicode = "f811";
    var svgPathData = "M114.3 93.7c-1.5 1.5-3.5 2.3-5.7 2.3L78.2 96c-8.8 0-16.1-7.2-13.8-15.8C73.6 44.8 112.8 0 190.2 0c58.4 0 95 25.5 113.5 53.2C323.6 40.6 350.2 32 384 32c77.4 0 116.6 44.8 125.8 80.2c2.2 8.6-5 15.8-13.8 15.8l-44.7 0c-2.1 0-4.2-.8-5.7-2.3l-24-24c-3.1-3.1-8.2-3.1-11.3 0l-24 24c-1.5 1.5-3.5 2.3-5.7 2.3l-79.2 0c28 79.9 21.2 170.2 9.9 226.1C370.9 365.1 416 417.3 416 480c0 17.7-14.3 32-32 32L64 512c-17.7 0-32-14.3-32-32c0-70.7 57.3-128 128-128l64 0c29.7-79.3 29.4-150.9 22.8-194l-87.6 87.6c-6.2 6.2-16.5 6.2-21-1.4c-18.5-31.6-22.5-90.9 32.3-145.7c1.4-1.4 2.8-2.7 4.1-4c-.3-.3-.7-.5-1-.8l-24-24c-3.1-3.1-8.2-3.1-11.3 0l-24 24z";
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
    exports.faIslandTropical = exports.definition;
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

// src/_entries/faIslandTropical.js
var import_faIslandTropical = __toESM(require_faIslandTropical());
var export_faIslandTropical = import_faIslandTropical.definition;
export {
  export_faIslandTropical as faIslandTropical
};
