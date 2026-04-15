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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCommentsQuestionCheck.js
var require_faCommentsQuestionCheck = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCommentsQuestionCheck.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "comments-question-check";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e14f";
    var svgPathData = "M208 352c114.9 0 208-78.8 208-176S322.9 0 208 0S0 78.8 0 176c0 38.6 14.7 74.3 39.6 103.4c-3.5 9.4-8.7 17.7-14.2 24.7c-4.8 6.2-9.7 11-13.3 14.3c-1.8 1.6-3.3 2.9-4.3 3.7c-.5 .4-.9 .7-1.1 .8l-.2 .2s0 0 0 0s0 0 0 0C1 327.2-1.4 334.4 .8 340.9S9.1 352 16 352c21.8 0 43.8-5.6 62.1-12.5c9.2-3.5 17.8-7.4 25.2-11.4C134.1 343.3 169.8 352 208 352zM448 176c0 112.3-99.1 196.9-216.5 207C255.8 457.4 336.4 512 432 512c38.2 0 73.9-8.7 104.7-23.9c7.5 4 16 7.9 25.2 11.4c18.3 6.9 40.3 12.5 62.1 12.5c6.9 0 13.1-4.5 15.2-11.1c2.1-6.6-.2-13.8-5.8-17.9c0 0 0 0 0 0s0 0 0 0l-.2-.2c-.2-.2-.6-.4-1.1-.8c-1-.8-2.5-2-4.3-3.7c-3.6-3.3-8.5-8.1-13.3-14.3c-5.5-7-10.7-15.4-14.2-24.7c24.9-29 39.6-64.7 39.6-103.4c0-92.8-84.9-168.9-192.6-175.5c.4 5.1 .6 10.3 .6 15.5zM180.2 77.3l48.5 0c29.1 0 52.6 23.6 52.6 52.6c0 18.8-10.1 36.3-26.4 45.6L228 191c-.2 10.9-9.1 19.6-20 19.6c-11 0-20-9-20-20l0-11.2c0-7.2 3.8-13.8 10-17.3L235 140.9c3.9-2.3 6.3-6.4 6.3-10.9c0-7-5.7-12.6-12.6-12.6l-48.5 0c-2.8 0-5.3 1.8-6.3 4.4l-.4 1c-3.7 10.4-15.1 15.9-25.5 12.2s-15.9-15.1-12.2-25.5l.4-1c6.6-18.6 24.2-31.1 44-31.1zm27.8 160a26.7 26.7 0 1 1 0 53.3 26.7 26.7 0 1 1 0-53.3zm358.1 88.8l-80 80c-7.8 7.8-20.5 7.8-28.3 0l-40-40c-7.8-7.8-7.8-20.5 0-28.3s20.5-7.8 28.3 0L472 363.7l65.9-65.9c7.8-7.8 20.5-7.8 28.3 0s7.8 20.5 0 28.3z";
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
    exports.faCommentsQuestionCheck = exports.definition;
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

// src/_entries/faCommentsQuestionCheck.js
var import_faCommentsQuestionCheck = __toESM(require_faCommentsQuestionCheck());
var export_faCommentsQuestionCheck = import_faCommentsQuestionCheck.definition;
export {
  export_faCommentsQuestionCheck as faCommentsQuestionCheck
};
