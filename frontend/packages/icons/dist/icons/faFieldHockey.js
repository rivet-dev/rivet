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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFieldHockeyStickBall.js
var require_faFieldHockeyStickBall = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFieldHockeyStickBall.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "field-hockey-stick-ball";
    var width = 512;
    var height = 512;
    var aliases = [127953, "field-hockey"];
    var unicode = "f44c";
    var svgPathData = "M491.2 8.5c21.8 15.1 27.3 45 12.2 66.8l-52.2 75.5L371.3 97.5l53.1-76.8c15.1-21.8 45-27.3 66.8-12.2zM353.1 123.9L433 177.2 241 454.9c-24.6 35.5-64.9 56.8-108.1 56.9C59.5 512 .1 452.3 .6 378.9l0-2.2c.2-25.7 7.8-50.8 21.8-72.4l17.3-26.5c14.5-22.2 44.2-28.4 66.4-13.9s28.4 44.2 13.9 66.4l-17.3 26.5c-4 6.1-6.2 13.3-6.2 20.6l0 2.2c-.1 20 16 36.2 36 36.1c11.7 0 22.7-5.8 29.4-15.5L353.1 123.9zM432 352a80 80 0 1 1 0 160 80 80 0 1 1 0-160z";
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
    exports.faFieldHockeyStickBall = exports.definition;
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

// src/_entries/faFieldHockey.js
var import_faFieldHockeyStickBall = __toESM(require_faFieldHockeyStickBall());
var export_faFieldHockey = import_faFieldHockeyStickBall.definition;
export {
  export_faFieldHockey as faFieldHockey
};
