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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFlatbreadStuffed.js
var require_faFlatbreadStuffed = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFlatbreadStuffed.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "flatbread-stuffed";
    var width = 576;
    var height = 512;
    var aliases = [129369];
    var unicode = "e40c";
    var svgPathData = "M272.2 4c-12.8-7-28.7-4.5-38.7 6.1L186.6 59.9c-4.6 4.9-10.5 8.2-17.1 9.5L102.4 82.7c-14.3 2.8-24.9 15-25.7 29.6l-3.8 68.3c-.4 6.7-2.8 13-7 18.2L23.1 252.1c-9.1 11.4-9.4 27.5-.7 39.2l40.9 54.8c4 5.3 6.2 11.8 6.4 18.5l.9 41.2c2.6-4.9 5.9-9.4 9.9-13.3l55.9-55.9C121.1 316.3 112 291.2 112 264c0-66.3 53.7-120 120-120c27.2 0 52.3 9.1 72.5 24.3L424.4 48.4c4.6-4.6 10-8.3 15.8-11.1c-6.1-3.9-13.4-5.7-21-4.8l-67.9 8.1c-6.6 .8-13.3-.5-19.2-3.7L272.2 4zM159.3 313.5L281.5 191.3C267.4 181.6 250.4 176 232 176c-48.6 0-88 39.4-88 88c0 18.4 5.6 35.4 15.3 49.5zM482.8 73.1c-4.3-5.4-10.6-8.6-17.4-9s-13.5 2.2-18.3 7l-43 43-43 43-86 86-86 86-86 86c-4.8 4.8-7.4 11.5-7 18.3s3.7 13.2 9 17.4C150.8 487.1 207.3 512 272 512c150.2 0 272-121.8 272-272c0-64.7-24.9-121.2-61.2-166.9zM256 432a16 16 0 1 1 32 0 16 16 0 1 1 -32 0zm80-16a16 16 0 1 1 0 32 16 16 0 1 1 0-32zm-48-48a16 16 0 1 1 32 0 16 16 0 1 1 -32 0z";
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
    exports.faFlatbreadStuffed = exports.definition;
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

// src/_entries/faFlatbreadStuffed.js
var import_faFlatbreadStuffed = __toESM(require_faFlatbreadStuffed());
var export_faFlatbreadStuffed = import_faFlatbreadStuffed.definition;
export {
  export_faFlatbreadStuffed as faFlatbreadStuffed
};
