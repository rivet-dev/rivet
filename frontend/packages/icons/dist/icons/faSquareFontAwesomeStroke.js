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

// src/node_modules/@fortawesome/free-brands-svg-icons/faSquareFontAwesomeStroke.js
var require_faSquareFontAwesomeStroke = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faSquareFontAwesomeStroke.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "square-font-awesome-stroke";
    var width = 448;
    var height = 512;
    var aliases = ["font-awesome-alt"];
    var unicode = "f35c";
    var svgPathData = "M64 64C46.3 64 32 78.3 32 96l0 320c0 17.7 14.3 32 32 32l320 0c17.7 0 32-14.3 32-32l0-320c0-17.7-14.3-32-32-32L64 64zM0 96C0 60.7 28.7 32 64 32l320 0c35.3 0 64 28.7 64 64l0 320c0 35.3-28.7 64-64 64L64 480c-35.3 0-64-28.7-64-64L0 96zm154 58c0 9.3-4.8 17.4-12.1 22l188.9 0c7.3 0 13.2 5.9 13.2 13.2 0 1.8-.4 3.7-1.1 5.4L312 264 342.9 333.4c.8 1.7 1.1 3.5 1.1 5.4 0 7.3-5.9 13.2-13.2 13.2l-186.8 0 0 32-32 0 0-209.5c-6.1-4.8-10-12.2-10-20.5 0-14.4 11.6-26 26-26s26 11.6 26 26z";
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
    exports.faSquareFontAwesomeStroke = exports.definition;
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

// src/_entries/faSquareFontAwesomeStroke.js
var import_faSquareFontAwesomeStroke = __toESM(require_faSquareFontAwesomeStroke());
var export_faSquareFontAwesomeStroke = import_faSquareFontAwesomeStroke.definition;
export {
  export_faSquareFontAwesomeStroke as faSquareFontAwesomeStroke
};
