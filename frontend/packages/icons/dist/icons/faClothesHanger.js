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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faClothesHanger.js
var require_faClothesHanger = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faClothesHanger.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "clothes-hanger";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e136";
    var svgPathData = "M209.8 84.8C222.3 35 267.1 0 318.5 0L320 0c61.9 0 112 50.1 112 112l0 6.3c0 36.5-17.8 70.8-47.8 91.8l-8.3 5.8L608.2 377.2C628.1 391 640 413.7 640 438c0 40.9-33.1 74-74 74L74 512c-40.9 0-74-33.1-74-74c0-24.2 11.9-47 31.8-60.8L301.6 189.8l.1-.1 45.8-32c12.8-9 20.5-23.7 20.5-39.3l0-6.3c0-26.5-21.5-48-48-48l-1.5 0c-22 0-41.2 15-46.6 36.4l-.9 3.4c-4.3 17.1-21.7 27.6-38.8 23.3s-27.6-21.7-23.3-38.8l.9-3.4zM320 255L68.3 429.7c-2.7 1.9-4.3 4.9-4.3 8.2c0 5.5 4.5 10 10 10l492 0c5.5 0 10-4.5 10-10c0-3.3-1.6-6.4-4.3-8.2L320 255z";
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
    exports.faClothesHanger = exports.definition;
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

// src/_entries/faClothesHanger.js
var import_faClothesHanger = __toESM(require_faClothesHanger());
var export_faClothesHanger = import_faClothesHanger.definition;
export {
  export_faClothesHanger as faClothesHanger
};
