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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMessagesQuestion.js
var require_faMessagesQuestion = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMessagesQuestion.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "messages-question";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e1e7";
    var svgPathData = "M0 64C0 28.7 28.7 0 64 0L352 0c35.3 0 64 28.7 64 64l0 192c0 35.3-28.7 64-64 64l-149.3 0-81.1 60.8c-4.8 3.6-11.3 4.2-16.8 1.5s-8.8-8.2-8.8-14.3l0-48-32 0c-35.3 0-64-28.7-64-64L0 64zM256 352l96 0c53 0 96-43 96-96l0-128 128 0c35.3 0 64 28.7 64 64l0 192c0 35.3-28.7 64-64 64l-32 0 0 48c0 6.1-3.4 11.6-8.8 14.3s-11.9 2.1-16.8-1.5L437.3 448 320 448c-35.3 0-64-28.7-64-64l0-32zM136.2 92.4l-.4 1c-3.7 10.4 1.8 21.8 12.2 25.5s21.8-1.8 25.5-12.2l.4-1c.9-2.7 3.5-4.4 6.3-4.4l48.5 0c7 0 12.6 5.7 12.6 12.6c0 4.5-2.4 8.7-6.3 10.9L198 146.1c-6.2 3.6-10 10.2-10 17.3l0 11.2c0 11 9 20 20 20c10.9 0 19.8-8.8 20-19.6l26.9-15.4c16.3-9.4 26.4-26.8 26.4-45.6c0-29.1-23.6-52.6-52.6-52.6l-48.5 0c-19.8 0-37.4 12.4-44 31.1zM234.7 248a26.7 26.7 0 1 0 -53.3 0 26.7 26.7 0 1 0 53.3 0z";
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
    exports.faMessagesQuestion = exports.definition;
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

// src/_entries/faMessagesQuestion.js
var import_faMessagesQuestion = __toESM(require_faMessagesQuestion());
var export_faMessagesQuestion = import_faMessagesQuestion.definition;
export {
  export_faMessagesQuestion as faMessagesQuestion
};
